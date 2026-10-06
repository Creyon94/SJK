//! Weather: the rain, snow, dust and mist a map asks for, presentation only.
//!
//! A map's weather entities (`fx_rain`, `fx_snow`, `fx_wind`, `fx_spacedust`) make the
//! server register effect names that start with `*` (`*heavyrain`, `*constantwind ( x y
//! z )`, ...). As cgame does (`CG_ParseWeatherEffect`), the client runs them as world
//! effect commands, in slot order ([`effects`]); `r_we` runs one more from the console.
//! A world without a server takes the names from its own entities.
//!
//! What SJK adds to the reference's particle clouds:
//!
//! - **Cover** ([`cover`], [`cover_map`]): weather exists only under open sky and above
//!   the first surface below it, surveyed from the map's collision data on a worker
//!   thread. Rain stops on roofs and the ground, stays out of buildings, and is cut
//!   exactly at eaves and windows, per pixel.
//! - **Splashes**: rain lands as small sprays on the ground and ripples on water.
//! - **Far rain**: a fainter second layer of streaks out to three times the reference's
//!   range, so a storm does not end a few metres away.
//! - Particles are generated on the GPU from their index and the wind the CPU
//!   integrates (`weather.wgsl`), lit by the light where the camera is, faded by the
//!   map's global fog, and drawn into the display-space effect layer after the effects,
//!   blended as the reference blends them.
//!
//! `r_weather 0` turns it all off; `r_weatherDensity` scales the particle counts
//! (1 is the reference's, SJK's default 2).

#[path = "weather_cover.rs"]
pub(crate) mod cover;
#[path = "weather_cover_map.rs"]
pub(crate) mod cover_map;
#[path = "weather_effects.rs"]
pub(crate) mod effects;
#[path = "weather_gpu.rs"]
pub(crate) mod gpu;
#[path = "weather_wind.rs"]
pub(crate) mod wind;

use bytemuck::Zeroable;
use effects::{Cloud, Effects, Look, MAX_CLOUDS};
use gpu::{BUCKETS, Batch, GpuCloud, GpuWeather, Kind};
use sjk_shell::{CvarDefinition, CvarError, CvarFlags, CvarRegistry, CvarValue};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Weather on (1) or off (0).
pub(crate) const CVAR: &str = "r_weather";
/// Particle count scale.
pub(crate) const DENSITY_CVAR: &str = "r_weatherDensity";
/// SJK's default density: twice the reference's particles (Sol asked for rain worth
/// looking at).
const DEFAULT_DENSITY: f32 = 2.0;
/// The console command running one world effect command (rd-vanilla's `r_we`).
pub(crate) const COMMAND: &str = "r_we";
pub(crate) const HELP: &str = "Run a weather command: rain, heavyrain, snow, fog, clear, ...";

/// `CS_EFFECTS` and `MAX_FX`.
const CS_EFFECTS: usize = 1355;
const MAX_FX: usize = 64;
/// Terminal speed of a weather particle per unit of force over mass: the reference adds
/// force / mass to the velocity and keeps 0.7 of it every frame (`mFrictionInverse`).
const TERMINAL: f64 = 0.7 / 0.3;
/// The far rain layer's reach and opacity, against the near box.
const FAR_SCALE: f32 = 3.0;
const FAR_OPACITY: f32 = 0.55;
/// Far streaks per near one: the far layer spreads over nine times the area.
const FAR_SHARE: f32 = 1.5;
/// Splashes per near rain streak.
const SPLASH_SHARE: f32 = 0.6;
/// Most particles in one batch (the instance number keeps 15 bits for them).
const MAX_BATCH: u32 = 0x7FFF;
/// Weather time wraps here, long before `f32` seconds lose precision.
const TIME_WRAP: f64 = 4096.0;

/// Live settings shared by the console and every installed world.
#[derive(Clone)]
pub(crate) struct Settings {
    enabled: Arc<AtomicU32>,
    density: Arc<AtomicU32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: Arc::new(AtomicU32::new(1)),
            density: Arc::new(AtomicU32::new(DEFAULT_DENSITY.to_bits())),
        }
    }
}

impl Settings {
    /// Register both archived cvars and follow their changes.
    pub(crate) fn bind(cvars: &mut CvarRegistry) -> Result<Self, CvarError> {
        cvars.register(CvarDefinition::new(
            CVAR,
            true,
            CvarFlags::ARCHIVE,
            "Rain, snow and mist on maps that have them",
        ))?;
        cvars.register(CvarDefinition::new(
            DENSITY_CVAR,
            f64::from(DEFAULT_DENSITY),
            CvarFlags::ARCHIVE,
            "Weather particle count, 1 as the original game, 0.25 to 4",
        ))?;
        let settings = Self::default();
        if let Some(cvar) = cvars.get(CVAR) {
            settings.set_enabled(&cvar.value);
        }
        if let Some(cvar) = cvars.get(DENSITY_CVAR) {
            settings.set_density(&cvar.value);
        }
        let changed = settings.clone();
        cvars.on_change(CVAR, move |change| changed.set_enabled(&change.current))?;
        let changed = settings.clone();
        cvars.on_change(DENSITY_CVAR, move |change| {
            changed.set_density(&change.current)
        })?;
        Ok(settings)
    }

    fn set_enabled(&self, value: &CvarValue) {
        let on = match value {
            CvarValue::Bool(value) => *value,
            CvarValue::Integer(value) => *value != 0,
            CvarValue::Float(value) => *value != 0.0,
            _ => return,
        };
        self.enabled.store(u32::from(on), Ordering::Relaxed);
    }

    fn set_density(&self, value: &CvarValue) {
        let value = match value {
            CvarValue::Float(value) => *value,
            CvarValue::Integer(value) => *value as f64,
            _ => return,
        };
        if value.is_finite() {
            let value = value.clamp(0.25, 4.0) as f32;
            self.density.store(value.to_bits(), Ordering::Relaxed);
        }
    }

    pub(crate) fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) != 0
    }

    pub(crate) fn density(&self) -> f32 {
        f32::from_bits(self.density.load(Ordering::Relaxed))
    }
}

/// The map's weather state, its cover and its GPU resources.
pub(crate) struct Runtime {
    /// The server's (or the map's) commands, without their `*`, in slot order.
    server: Vec<String>,
    /// `r_we` commands typed since the map loaded, run after the server's.
    local: Vec<String>,
    effects: Effects,
    /// The map's `misc_weather_zone` boxes.
    map_zones: Vec<[[f32; 3]; 2]>,
    /// The zones the running survey was started with.
    surveyed_zones: Option<Vec<[[f32; 3]; 2]>>,
    wind: wind::Wind,
    /// Per cloud, the force integrated over time (units·s per unit of mass).
    flows: [[f64; 3]; MAX_CLOUDS],
    time: f64,
    light: [f32; 3],
    last_frame: Option<std::time::Instant>,
    cover: Option<cover_map::CoverMap>,
    gpu: Option<gpu::Gpu>,
    uniform: GpuWeather,
    batches: Vec<Batch>,
}

/// What the frame needs from the rest of the client.
pub(crate) struct FrameInput<'a> {
    pub(crate) device: &'a wgpu::Device,
    pub(crate) queue: &'a crate::frame_queue::FrameQueue,
    pub(crate) camera_layout: &'a wgpu::BindGroupLayout,
    pub(crate) images: Option<(
        &'a sjk_vfs::VirtualFileSystem,
        &'a sjk_shader::ShaderCatalog,
    )>,
    pub(crate) bsp: &'a Arc<sjk_bsp::Bsp>,
    pub(crate) camera: [f32; 3],
    /// Light at the camera, 0..1 per channel (ambient and directed).
    pub(crate) light: [f32; 3],
    /// The global fog's `1 / depthForOpaque`, 0 without one.
    pub(crate) fog: f32,
    pub(crate) viewport: [u32; 2],
    pub(crate) settings: &'a Settings,
}

impl Runtime {
    /// The weather of a world being installed: the gamestate's effect names, or without
    /// one the map's own weather entities.
    pub(crate) fn new(game: Option<&sjk_protocol::GameState>, bsp: &sjk_bsp::Bsp) -> Self {
        let entities = sjk_entity::parse_entity_lump(bsp.entities()).unwrap_or_default();
        let server = match game {
            Some(game) => server_commands(game),
            None => entities
                .iter()
                .flat_map(sjk_game_jka::map_scenery::weather_effects)
                .filter_map(|name| name.strip_prefix('*').map(str::to_owned))
                .collect(),
        };
        let map_zones = entities
            .iter()
            .filter(|entity| {
                entity
                    .classname()
                    .is_some_and(|name| name.eq_ignore_ascii_case("misc_weather_zone"))
            })
            .filter_map(|entity| {
                let model = entity.get("model")?.strip_prefix('*')?.parse().ok()?;
                let model = bsp.render().inline_model(model)?;
                Some([model.minimums, model.maximums])
            })
            .collect();
        let effects = Effects::from_commands(server.iter().map(String::as_str));
        if !effects.is_empty() {
            crate::log::progress(format_args!("weather: {}", server.join(", ")));
        }
        Self {
            server,
            local: Vec::new(),
            effects,
            map_zones,
            surveyed_zones: None,
            wind: wind::Wind::default(),
            flows: [[0.0; 3]; MAX_CLOUDS],
            time: 0.0,
            light: [1.0; 3],
            last_frame: None,
            cover: None,
            gpu: None,
            uniform: GpuWeather::zeroed(),
            batches: Vec::with_capacity(2 * MAX_CLOUDS + 1),
        }
    }

    /// The server's effect names changed: rerun every command if its weather did.
    pub(crate) fn refresh_server(&mut self, game: &sjk_protocol::GameState) {
        let server = server_commands(game);
        if server != self.server {
            self.server = server;
            self.rebuild();
        }
    }

    /// `r_we <command>`.
    pub(crate) fn command(&mut self, command: &str) -> Result<(), &'static str> {
        let mut probe = self.effects.clone();
        probe.apply(command)?;
        self.local.push(command.to_owned());
        self.rebuild();
        Ok(())
    }

    fn rebuild(&mut self) {
        let commands = self.server.iter().chain(&self.local).map(String::as_str);
        self.effects = Effects::from_commands(commands);
        self.flows = [[0.0; 3]; MAX_CLOUDS];
    }

    /// Something to draw this frame.
    pub(crate) fn visible(&self) -> bool {
        !self.batches.is_empty()
    }

    /// Advance the weather and write this frame's uniform. Allocation-free once the
    /// weather is set up, unless the cover window moves.
    pub(crate) fn prepare(&mut self, input: FrameInput<'_>) {
        self.batches.clear();
        let now = std::time::Instant::now();
        let seconds = self
            .last_frame
            .replace(now)
            .map_or(0.0, |last| now.duration_since(last).as_secs_f32().min(0.25));
        if !input.settings.enabled() || self.effects.clouds.is_empty() {
            return;
        }
        let surveyed = self.surveyed_zones.as_deref().is_some_and(|surveyed| {
            surveyed
                .iter()
                .eq(self.map_zones.iter().chain(&self.effects.zones))
        });
        if self.gpu.is_none() || !surveyed {
            let zones = self.zones();
            let cover = self
                .cover
                .get_or_insert_with(|| cover_map::CoverMap::new(input.device));
            cover.start(input.bsp.clone(), cover::Marks::read(input.bsp, &zones));
            self.surveyed_zones = Some(zones);
            if self.gpu.is_none() {
                self.gpu = Some(gpu::Gpu::new(
                    input.device,
                    input.queue,
                    input.images,
                    input.camera_layout,
                    &cover.view,
                ));
            }
        }
        let (Some(cover), Some(gpu)) = (self.cover.as_mut(), self.gpu.as_ref()) else {
            return;
        };
        cover.update(input.queue, input.camera);

        let wind = self.wind.advance(&mut self.effects.winds, seconds);
        if !self.effects.frozen {
            self.time = (self.time + f64::from(seconds)) % TIME_WRAP;
            for (flow, cloud) in self.flows.iter_mut().zip(&self.effects.clouds) {
                let force = force(cloud, wind);
                for axis in 0..3 {
                    flow[axis] += f64::from(force[axis]) * f64::from(seconds);
                }
            }
        }
        // Light changes ease over about half a second, so walking under a lamp does not
        // flicker the rain.
        let target = input.light.map(|value| (value / 0.7).clamp(0.3, 1.0));
        let ease = 1.0 - (-seconds / 0.4).exp();
        for (light, target) in self.light.iter_mut().zip(target) {
            *light += (target - *light) * if seconds == 0.0 { 1.0 } else { ease };
        }

        let window = cover.window();
        self.uniform.window = window.cells;
        self.uniform.cover = [
            cover_map::CELL,
            cover_map::SIZE as f32,
            f32::from(u8::from(window.enabled)),
            self.time as f32,
        ];
        let density = input.settings.density();
        let mut splash = None;
        for (slot, cloud) in self.effects.clouds.iter().enumerate() {
            let near = ((cloud.count as f32 * density).round() as u32).min(MAX_BATCH);
            let far = if cloud.is_rain() {
                ((near as f32 * FAR_SHARE).round() as u32).min(MAX_BATCH)
            } else {
                0
            };
            self.uniform.clouds[slot] = gpu_cloud(cloud, wind, &self.flows[slot]);
            if near == 0 {
                continue;
            }
            let base = (slot as u32) << 16;
            let kind = match cloud.look {
                Look::Streak => Kind::Streak,
                Look::Sprite(_) if cloud.additive => Kind::Sprite,
                Look::Sprite(_) => Kind::SpriteAlpha,
            };
            self.batches.push(Batch {
                kind,
                instances: base..base + near,
            });
            if far != 0 {
                let base = base | 0x8000;
                self.batches.push(Batch {
                    kind,
                    instances: base..base + far,
                });
            }
            if cloud.is_rain() && splash.is_none() {
                splash = Some((slot, ((near as f32 * SPLASH_SHARE) as u32).min(MAX_BATCH)));
            }
        }
        if let Some((slot, count)) = splash.filter(|&(_, count)| count != 0 && window.enabled) {
            let base = 7 << 16;
            self.batches.push(Batch {
                kind: Kind::Splash,
                instances: base..base + count,
            });
            self.uniform.light[3] = slot as f32;
        }
        self.uniform.view = [
            input.viewport[0] as f32,
            input.viewport[1] as f32,
            input.fog,
            0.0,
        ];
        self.uniform.light[..3].copy_from_slice(&self.light);
        gpu.write(input.queue, &self.uniform);
    }

    /// Record this frame's weather into the effect layer's pass.
    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        depth: &'pass wgpu::BindGroup,
    ) {
        if let Some(gpu) = &self.gpu {
            gpu.draw(pass, camera, depth, &self.batches);
        }
    }

    fn zones(&self) -> Vec<[[f32; 3]; 2]> {
        self.map_zones
            .iter()
            .chain(&self.effects.zones)
            .copied()
            .collect()
    }
}

/// The `*` effect names of the gamestate, in slot order, without the `*`.
fn server_commands(game: &sjk_protocol::GameState) -> Vec<String> {
    (1..MAX_FX)
        .filter_map(|slot| game.config_string(CS_EFFECTS + slot))
        .filter_map(|bytes| bytes.strip_prefix(b"*"))
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
        .collect()
}

/// Gravity and wind on a cloud's particles.
fn force(cloud: &Cloud, wind: [f32; 3]) -> [f32; 3] {
    [wind[0], wind[1], wind[2] - cloud.gravity]
}

/// A cloud as the shader reads it: its look, its box, and the velocity and wrapped flow
/// of each mass bucket.
fn gpu_cloud(cloud: &Cloud, wind: [f32; 3], flow: &[f64; 3]) -> GpuCloud {
    let [low, high] = cloud.range;
    let size: [f32; 3] = std::array::from_fn(|axis| (high[axis] - low[axis]).max(1.0));
    let force = force(cloud, wind);
    let mut result = GpuCloud {
        color: cloud.color,
        shape: match cloud.look {
            Look::Streak => [cloud.width * 0.5, cloud.height, 0.0, 0.0],
            Look::Sprite(image) => [
                cloud.width,
                cloud.height,
                image as u32 as f32,
                f32::from(u8::from(cloud.rotates)),
            ],
        },
        box_min: [low[0], low[1], low[2], 0.0],
        box_size: [size[0], size[1], size[2], 0.0],
        layers: [FAR_SCALE, FAR_OPACITY, 0.0, 0.0],
        velocity: [[0.0; 4]; BUCKETS],
        offset: [[0.0; 4]; BUCKETS],
    };
    // The flow wraps to the far box across and the box up and down: whole multiples of
    // the near box, so both layers read the same offset.
    let wrap = [
        f64::from(size[0] * FAR_SCALE),
        f64::from(size[1] * FAR_SCALE),
        f64::from(size[2]),
    ];
    for bucket in 0..BUCKETS {
        let share = (bucket as f32 + 0.5) / BUCKETS as f32;
        let mass = f64::from(cloud.mass[0] + (cloud.mass[1] - cloud.mass[0]) * share).max(0.001);
        for axis in 0..3 {
            result.velocity[bucket][axis] = (TERMINAL * f64::from(force[axis]) / mass) as f32;
            result.offset[bucket][axis] =
                (TERMINAL * flow[axis] / mass).rem_euclid(wrap[axis]) as f32;
        }
    }
    result
}

/// C `atoi`: the leading integer, 0 without one.
pub(crate) fn atoi(text: &str) -> i64 {
    let text = text.trim_start();
    let (sign, digits) = match text.as_bytes().first() {
        Some(b'-') => (-1, &text[1..]),
        Some(b'+') => (1, &text[1..]),
        _ => (1, text),
    };
    let end = digits
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(digits.len());
    sign * digits[..end].parse::<i64>().unwrap_or(0)
}

/// C `atof`: the longest leading number, 0 without one.
pub(crate) fn atof(text: &str) -> f32 {
    let text = text.trim_start();
    (1..=text.len())
        .rev()
        .filter(|&end| text.is_char_boundary(end))
        .find_map(|end| text[..end].parse::<f32>().ok())
        .filter(|value| value.is_finite())
        .unwrap_or(0.0)
}

impl crate::GpuState {
    /// Advance the weather for the main view about to be drawn.
    pub(crate) fn prepare_weather(&mut self, camera: &crate::camera_uniform::CameraUniform) {
        let light = self
            .entity_lighting
            .sample(&self.bsp, camera.camera_position, &[]);
        let light: [f32; 3] =
            std::array::from_fn(|channel| light.ambient[channel] + 0.5 * light.directed[channel]);
        let fogs = self.world_materials.fogs();
        let fog = if self.world_materials.fog_mode == crate::fog_volumes::Mode::Off {
            0.0
        } else {
            fogs.entries[1..=fogs.count]
                .iter()
                .find(|fog| fog.color[3] != 0.0)
                .map_or(0.0, |fog| fog.bounds_min[3] * 8.0)
        };
        let viewport = self
            .post_aa
            .as_ref()
            .and_then(|aa| aa.effect_layer())
            .map_or(
                [self.configuration.width, self.configuration.height],
                |layer| layer.size(),
            );
        let vfs = self.vfs.clone();
        self.weather.prepare(FrameInput {
            device: &self.device,
            queue: &self.queue,
            camera_layout: &self.camera_layout,
            images: vfs.as_deref().map(|vfs| (vfs, &self.shaders)),
            bsp: &self.bsp,
            camera: camera.camera_position,
            light,
            fog,
            viewport,
            settings: &self.context.weather,
        });
    }

    /// `r_we`: run one world effect command for this map.
    pub(crate) fn weather_command(&mut self, args: &[String]) -> Result<Vec<String>, String> {
        let command = args.join(" ");
        self.weather
            .command(&command)
            .map(|()| Vec::new())
            .map_err(str::to_owned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_number_parsing_takes_the_leading_number() {
        assert_eq!(atoi("300"), 300);
        assert_eq!(atoi("  -12abc"), -12);
        assert_eq!(atoi("x"), 0);
        assert_eq!(atof("-5000.000000"), -5000.0);
        assert_eq!(atof("1.5e2x"), 150.0);
        assert_eq!(atof("nope"), 0.0);
    }

    #[test]
    fn rain_falls_at_the_reference_terminal_speed_and_both_layers_share_the_flow() {
        let effects = Effects::from_commands(["heavyrain"]);
        let cloud = &effects.clouds[0];
        let wind = [-5000.0, 0.0, 0.0];
        let flow = [-123_456.0, 7.0, -98_765.0];
        let gpu = gpu_cloud(cloud, wind, &flow);
        // The lightest bucket: mass 5.3125, so 2800 / m × 7/3 downwards.
        let mass = 5.0 + 5.0 * 0.5 / BUCKETS as f32;
        let expected = -(TERMINAL as f32) * 2800.0 / mass;
        assert!((gpu.velocity[0][2] - expected).abs() < 0.01);
        assert!(gpu.velocity[0][0] < -2000.0, "the wind carries it");
        for bucket in gpu.offset {
            assert!((0.0..1250.0 * FAR_SCALE).contains(&bucket[0]));
            assert!((0.0..1250.0).contains(&bucket[2]));
        }
        assert_eq!(gpu.shape[..2], [0.6, 80.0]);
    }

    #[test]
    fn worlds_without_a_server_take_their_own_weather_entities() {
        use sjk_bsp::{Bsp, CollisionShader, box_brush, write_collision_map_with_models};
        let shaders = [CollisionShader {
            name: "textures/stone".into(),
            surface_flags: 0,
            content_flags: 1,
        }];
        let entities = "{\n\"classname\" \"worldspawn\"\n}\n\
            {\n\"classname\" \"fx_rain\"\n\"spawnflags\" \"20\"\n}\n\
            {\n\"classname\" \"fx_wind\"\n\"spawnflags\" \"2\"\n\"angle\" \"180\"\n\"speed\" \"5000\"\n}\n\
            {\n\"classname\" \"misc_weather_zone\"\n\"model\" \"*1\"\n}\n";
        let zone = vec![box_brush([0.0; 3], [64.0, 32.0, 16.0], 0)];
        let data = write_collision_map_with_models(
            entities,
            &shaders,
            &[box_brush([-8.0; 3], [8.0; 3], 0)],
            &[zone],
        );
        let bsp = Bsp::parse(&data).unwrap();
        let weather = Runtime::new(None, &bsp);
        assert_eq!(weather.server[..2], ["heavyrain", "heavyrainfog"]);
        assert!(weather.server[2].starts_with("constantwind ( -5000.000000"));
        assert_eq!(weather.effects.clouds.len(), 2);
        assert!((weather.effects.winds[0].current()[0] + 5000.0).abs() < 0.01);
        assert_eq!(weather.map_zones, [[[0.0; 3], [64.0, 32.0, 16.0]]]);
    }

    #[test]
    fn console_commands_add_to_the_maps_and_reject_unknown_ones() {
        let bsp = sjk_bsp::Bsp::empty([-64.0; 3], [64.0; 3]);
        let mut weather = Runtime::new(None, &bsp);
        assert!(weather.effects.is_empty());
        assert_eq!(weather.command("snow"), Ok(()));
        assert_eq!(weather.command("hail"), Err(effects::HELP));
        assert_eq!(weather.local, ["snow"]);
        assert_eq!(weather.effects.clouds.len(), 1);
        assert_eq!(weather.command("clear"), Ok(()));
        assert!(weather.effects.clouds.is_empty());
    }
}
