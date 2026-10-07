//! World notes: in-game annotations of surfaces and objects for whoever fixes them.
//!
//! With no player card pinned and no player under the crosshair, `inspect` selects the
//! world surface under the crosshair, or the mover whose bounds the view ray meets
//! first, and names it in a centre print. A second `inspect` within
//! [`CONFIRM_MS`] confirms the selection and opens a note line in the chat composer.
//! Enter appends the note with the selection to `notes.jsonl` (one JSON object per
//! line) and a readable line to `notes.txt` in the config directory, and takes a
//! JPEG screenshot of the same view named after the note. Escape drops the note.
//!
//! While a selection waits or its note is written, its outline is drawn over the view
//! as flickering green dots along the surface's triangle edges (a mover's bounds),
//! projected each frame, at most [`HIGHLIGHT_DOTS`] of them.
//!
//! A selection records what a fix needs: the map, the camera pose and a `setviewpos`
//! back to it, the hit point and normal, the BSP draw surface (index, shader, kind,
//! lightmap or vertex lighting, BSP material), the collision trace's surface flags, and
//! the nearest map entity (an inline model whose bounds hold the hit, or the closest
//! origin within [`ENTITY_RADIUS`]).

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use sjk_bsp::{Aabb, Bsp, TraceScratch};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

/// How long a selection waits for its confirming press.
pub(crate) const CONFIRM_MS: u128 = 20_000;
/// Farthest a selection reaches.
const REACH: f32 = 16_384.0;
/// Entities whose origin lies this close to the hit are named when no inline model
/// holds it.
const ENTITY_RADIUS: f32 = 160.0;
/// `CONTENTS_SOLID | CONTENTS_PLAYERCLIP`-style mask of the collision trace: anything a
/// player could stand against.
const COLLISION_MASK: u32 = 1 | 0x10000;
/// JKA's standing view height: `setviewpos` places the origin, the eye sits above it.
const VIEW_HEIGHT: f32 = 36.0;
/// Most edges an outline keeps, and most dots it draws a frame (the HUD's shapes share
/// 16,384 vertices, six a dot).
const HIGHLIGHT_EDGES: usize = 512;
pub(crate) const HIGHLIGHT_DOTS: usize = 700;

/// What `inspect` on the world selected.
#[derive(Clone, Debug)]
pub(crate) struct Selection {
    map: String,
    pose: String,
    eye: Vec3,
    forward: Vec3,
    hit: Vec3,
    normal: Vec3,
    distance: f32,
    surface: Option<Surface>,
    entity: Option<Entity>,
    collision: Option<Collision>,
    /// The outline drawn while selected (`Notes::draw_highlight`).
    edges: Vec<[Vec3; 2]>,
    selected: Instant,
}

#[derive(Clone, Debug)]
struct Surface {
    index: usize,
    shader: String,
    kind: &'static str,
    lightmap: i32,
    vertices: usize,
    material: u32,
}

#[derive(Clone, Debug)]
struct Entity {
    number: usize,
    classname: String,
    targetname: String,
    model: String,
    origin: String,
    /// The inline model's shaders, for a mover the ray met.
    shaders: Vec<String>,
}

#[derive(Clone, Debug)]
struct Collision {
    distance: f32,
    surface_flags: u32,
    content_flags: u32,
}

impl Selection {
    /// One line naming the selection, for the centre print and the note's subject.
    pub(crate) fn summary(&self) -> String {
        let what = match (&self.surface, &self.entity) {
            (_, Some(entity)) if !entity.shaders.is_empty() => format!(
                "{} {} ({})",
                entity.classname,
                entity.model,
                entity.shaders.first().map_or("", String::as_str)
            ),
            (Some(surface), _) => format!(
                "{} (surface {}, {})",
                surface.shader,
                surface.index,
                lighting(surface.lightmap)
            ),
            (None, Some(entity)) => entity.classname.clone(),
            (None, None) => "nothing".to_owned(),
        };
        format!("{what}, {:.0} units", self.distance)
    }

    fn fresh(&self, now: Instant) -> bool {
        now.duration_since(self.selected).as_millis() <= CONFIRM_MS
    }

    /// `setviewpos` that puts the eye back where the selection was made.
    fn setviewpos(&self) -> String {
        let yaw = self
            .forward
            .y
            .atan2(self.forward.x)
            .to_degrees()
            .rem_euclid(360.0);
        format!(
            "setviewpos {:.0} {:.0} {:.0} {:.0}",
            self.eye.x,
            self.eye.y,
            self.eye.z - VIEW_HEIGHT,
            yaw
        )
    }

    fn json(&self, note: &str, time: &str, screenshot: &str) -> serde_json::Value {
        let vector = |v: Vec3| serde_json::json!([round(v.x), round(v.y), round(v.z)]);
        serde_json::json!({
            "time": time,
            "map": self.map,
            "note": note,
            "selection": self.summary(),
            "setviewpos": self.setviewpos(),
            "pose": self.pose,
            "hit": vector(self.hit),
            "normal": vector(self.normal),
            "distance": round(self.distance),
            "surface": self.surface.as_ref().map(|s| serde_json::json!({
                "index": s.index,
                "shader": s.shader,
                "kind": s.kind,
                "lightmap": s.lightmap,
                "lighting": lighting(s.lightmap),
                "vertices": s.vertices,
                "bsp_material": s.material,
            })),
            "entity": self.entity.as_ref().map(|e| serde_json::json!({
                "number": e.number,
                "classname": e.classname,
                "targetname": e.targetname,
                "model": e.model,
                "origin": e.origin,
                "shaders": e.shaders,
            })),
            "collision": self.collision.as_ref().map(|c| serde_json::json!({
                "distance": round(c.distance),
                "surface_flags": format!("{:#x}", c.surface_flags),
                "content_flags": format!("{:#x}", c.content_flags),
            })),
            "screenshot": screenshot,
        })
    }
}

fn lighting(lightmap: i32) -> &'static str {
    match lightmap {
        0.. => "lightmapped",
        -3 => "vertex-lit",
        -2 => "white image",
        _ => "unlit",
    }
}

fn round(value: f32) -> f64 {
    (f64::from(value) * 10.0).round() / 10.0
}

/// The selection under the view ray of `camera`: the nearest draw surface of the world,
/// unless a mover's bounds come first.
pub(crate) fn pick(
    bsp: &Bsp,
    surfaces: &crate::decal_marks::DecalSurfaces,
    scratch: &mut TraceScratch,
    camera: &CameraUniform,
    map: &str,
) -> Option<Selection> {
    let eye = Vec3::from_array(camera.camera_position);
    let forward = Vec3::from_array(camera.view_forward).normalize_or_zero();
    if forward == Vec3::ZERO {
        return None;
    }
    let world = surfaces.ray_hit(eye, forward, REACH);
    let entities = sjk_entity::parse_entity_lump(bsp.entities()).unwrap_or_default();
    // Movers are not in the world's triangles: the first inline model box the ray meets.
    let mut mover = None;
    for (number, entity) in entities.iter().enumerate() {
        let Some(index) = entity
            .get("model")
            .and_then(|model| model.strip_prefix('*'))
            .and_then(|index| index.parse::<usize>().ok())
        else {
            continue;
        };
        let Some(model) = bsp.render().inline_model(index) else {
            continue;
        };
        let origin = parse_vector(entity.get("origin")).unwrap_or(Vec3::ZERO);
        let low = Vec3::from_array(model.minimums) + origin;
        let high = Vec3::from_array(model.maximums) + origin;
        let Some(distance) = ray_box(eye, forward, low, high) else {
            continue;
        };
        if world.as_ref().is_some_and(|hit| hit.distance < distance)
            || mover
                .as_ref()
                .is_some_and(|(_, _, nearest)| *nearest <= distance)
        {
            continue;
        }
        mover = Some((number, index, distance));
    }
    let trace = bsp.trace_box_with(
        scratch,
        eye.to_array(),
        (eye + forward * REACH).to_array(),
        Aabb::new([0.0; 3], [0.0; 3]).expect("point bounds are valid"),
        COLLISION_MASK,
    );
    let collision = (trace.fraction < 1.0).then_some(Collision {
        distance: trace.fraction * REACH,
        surface_flags: trace.surface_flags,
        content_flags: trace.content_flags,
    });
    let render = bsp.render();
    let shader_name = |index: usize| {
        bsp.shaders().get(index).map_or_else(
            || format!("shader {index}"),
            |s| s.name_lossy().into_owned(),
        )
    };
    let entity_of = |number: usize, shaders: Vec<String>| {
        let entity = &entities[number];
        let text = |key: &str| entity.get(key).unwrap_or("").to_owned();
        Entity {
            number,
            classname: text("classname"),
            targetname: text("targetname"),
            model: text("model"),
            origin: text("origin"),
            shaders,
        }
    };
    let selection = |hit: Vec3, normal: Vec3, distance: f32, surface, entity, edges| Selection {
        map: map.to_owned(),
        pose: camera.viewpos(),
        eye,
        forward,
        hit,
        normal,
        distance,
        surface,
        entity,
        collision: collision.clone(),
        edges,
        selected: Instant::now(),
    };
    if let Some((number, index, distance)) = mover {
        let shaders = render
            .inline_model(index)
            .map(|model| {
                let mut names: Vec<String> = model
                    .surfaces
                    .clone()
                    .filter_map(|surface| render.surfaces().get(surface))
                    .map(|surface| shader_name(surface.shader))
                    .collect();
                names.sort();
                names.dedup();
                names
            })
            .unwrap_or_default();
        let hit = eye + forward * distance;
        let edges = render.inline_model(index).map_or_else(Vec::new, |model| {
            let origin = parse_vector(entities[number].get("origin")).unwrap_or(Vec3::ZERO);
            box_edges(
                Vec3::from_array(model.minimums) + origin,
                Vec3::from_array(model.maximums) + origin,
            )
        });
        return Some(selection(
            hit,
            -forward,
            distance,
            None,
            Some(entity_of(number, shaders)),
            edges,
        ));
    }
    let hit = world?;
    let point = eye + forward * hit.distance;
    let surface = render.surfaces().get(hit.surface).map(|surface| {
        let shader = bsp.shaders().get(surface.shader);
        Surface {
            index: hit.surface,
            shader: shader_name(surface.shader),
            kind: match surface.kind {
                sjk_bsp::SurfaceKind::Planar => "planar",
                sjk_bsp::SurfaceKind::Patch => "patch",
                sjk_bsp::SurfaceKind::TriangleSoup => "triangle soup",
                sjk_bsp::SurfaceKind::Flare => "flare",
            },
            lightmap: surface.lightmaps[0],
            vertices: surface.vertices.len(),
            material: shader.map_or(0, |s| s.surface_flags & 0x1f),
        }
    });
    let entity = nearest_entity(&entities, point).map(|number| entity_of(number, Vec::new()));
    let edges = surfaces.surface_edges(hit.surface, HIGHLIGHT_EDGES);
    Some(selection(
        point,
        hit.normal,
        hit.distance,
        surface,
        entity,
        edges,
    ))
}

/// The entity, other than the world, whose origin is nearest `point` within
/// [`ENTITY_RADIUS`].
fn nearest_entity(entities: &[sjk_entity::Entity], point: Vec3) -> Option<usize> {
    entities
        .iter()
        .enumerate()
        .skip(1)
        .filter_map(|(number, entity)| {
            let origin = parse_vector(entity.get("origin"))?;
            let distance = origin.distance(point);
            (distance <= ENTITY_RADIUS).then_some((number, distance))
        })
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(number, _)| number)
}

fn parse_vector(text: Option<&str>) -> Option<Vec3> {
    let mut parts = text?
        .split_whitespace()
        .map(|part| part.parse::<f32>().ok());
    Some(Vec3::new(parts.next()??, parts.next()??, parts.next()??))
}

/// The twelve edges of a box.
fn box_edges(low: Vec3, high: Vec3) -> Vec<[Vec3; 2]> {
    let corner = |i: usize| {
        Vec3::new(
            if i & 1 == 0 { low.x } else { high.x },
            if i & 2 == 0 { low.y } else { high.y },
            if i & 4 == 0 { low.z } else { high.z },
        )
    };
    (0..8)
        .flat_map(|i| [1, 2, 4].map(|bit| (i, i | bit)))
        .filter(|(from, to)| from != to)
        .map(|(from, to)| [corner(from), corner(to)])
        .collect()
}

/// `point` on the screen of `view_projection` (physical pixels), if in front of the eye.
fn project(view_projection: &glam::Mat4, viewport: [f32; 2], point: Vec3) -> Option<[f32; 2]> {
    let clip = *view_projection * point.extend(1.0);
    if clip.w <= 0.5 {
        return None;
    }
    let (x, y) = (clip.x / clip.w, clip.y / clip.w);
    (x.abs() <= 1.2 && y.abs() <= 1.2)
        .then(|| [(x + 1.0) * 0.5 * viewport[0], (1.0 - y) * 0.5 * viewport[1]])
}

/// Entry distance of the ray into the box, if it meets it ahead.
fn ray_box(origin: Vec3, direction: Vec3, low: Vec3, high: Vec3) -> Option<f32> {
    let inverse = direction.recip();
    let a = (low - origin) * inverse;
    let b = (high - origin) * inverse;
    let near = a.min(b).max_element();
    let far = a.max(b).min_element();
    (far >= near.max(0.0)).then_some(near.max(0.0))
}

/// The selection waiting for its confirming press, and the note being written.
pub(crate) struct Notes {
    selection: Option<Selection>,
    /// The confirmed selection the composer's note belongs to.
    writing: Option<Selection>,
    /// The last rendered main view.
    camera: Option<CameraUniform>,
    /// The selection's outline for this frame.
    highlight: sjk_ui::DrawList,
    started: Instant,
}

impl Default for Notes {
    fn default() -> Self {
        Self {
            selection: None,
            writing: None,
            camera: None,
            highlight: sjk_ui::DrawList::new(HIGHLIGHT_DOTS),
            started: Instant::now(),
        }
    }
}

/// What an `inspect` press on the world did.
pub(crate) enum Press {
    /// Selected (or reselected) something: show its summary.
    Selected(String),
    /// Confirmed the selection: open the composer for a note about it.
    Confirmed(String),
    /// Nothing under the crosshair.
    Nothing,
}

impl Notes {
    /// Keep the final main view (`upload_scene_camera`).
    pub(crate) fn set_camera(&mut self, camera: CameraUniform) {
        self.camera = Some(camera);
    }

    /// An `inspect` press with no player involved.
    pub(crate) fn press(
        &mut self,
        bsp: &Bsp,
        surfaces: &crate::decal_marks::DecalSurfaces,
        scratch: &mut TraceScratch,
        map: &str,
    ) -> Press {
        let now = Instant::now();
        if let Some(selection) = self.selection.take().filter(|s| s.fresh(now)) {
            let subject = selection.summary();
            self.writing = Some(selection);
            return Press::Confirmed(subject);
        }
        let Some(camera) = self.camera else {
            return Press::Nothing;
        };
        match pick(bsp, surfaces, scratch, &camera, map) {
            Some(selection) => {
                let summary = selection.summary();
                self.selection = Some(selection);
                Press::Selected(summary)
            }
            None => Press::Nothing,
        }
    }

    /// The composer closed without saving (Escape): forget the confirmed selection.
    pub(crate) fn composer_closed(&mut self) {
        self.writing = None;
    }

    /// Rebuild the outline of the waiting or confirmed selection for this frame.
    pub(crate) fn draw_highlight(&mut self, viewport: [f32; 2]) {
        self.highlight.clear();
        let now = Instant::now();
        if self.selection.as_ref().is_some_and(|s| !s.fresh(now)) {
            self.selection = None;
        }
        let (Some(selection), Some(camera)) = (
            self.writing.as_ref().or(self.selection.as_ref()),
            self.camera,
        ) else {
            return;
        };
        let view_projection = glam::Mat4::from_cols_array_2d(&camera.view_projection);
        // Space the dots by the outline's length on screen, so a big surface keeps
        // within the budget and a small one still reads as a line.
        let lengths: Vec<f32> = selection
            .edges
            .iter()
            .map(|[a, b]| {
                match (
                    project(&view_projection, viewport, *a),
                    project(&view_projection, viewport, *b),
                ) {
                    (Some(a), Some(b)) => ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt(),
                    _ => 120.0,
                }
            })
            .collect();
        let total: f32 = lengths.iter().sum();
        let spacing = (total / HIGHLIGHT_DOTS as f32).max(7.0);
        let time = now.duration_since(self.started).as_secs_f32();
        let size = (viewport[1] / 540.0).clamp(2.0, 4.0);
        let mut index = 0u32;
        for ([a, b], length) in selection.edges.iter().zip(&lengths) {
            let steps = ((length / spacing).ceil() as usize).clamp(1, 64);
            for step in 0..=steps {
                let point = a.lerp(*b, step as f32 / steps as f32);
                index = index.wrapping_add(1);
                let Some([x, y]) = project(&view_projection, viewport, point) else {
                    continue;
                };
                // Each dot flickers on its own beat, brighter as a wave runs along.
                let seed = (index as f32 * 12.9898 + (time * 9.0).floor() * 78.233).sin();
                let flicker = (seed * 43_758.547).fract().abs();
                let wave = ((index as f32 * 0.21 - time * 6.0).sin() * 0.5 + 0.5).powi(4);
                let alpha = (0.35 + 0.4 * flicker + 0.25 * wave).min(1.0);
                if !self.highlight.push(sjk_ui::DrawCommand::SolidRect {
                    rect: sjk_ui::Rect::new(x - size * 0.5, y - size * 0.5, size, size),
                    color: sjk_ui::Color::new(0.25, 1.0, 0.45, alpha),
                }) {
                    return;
                }
            }
        }
    }

    /// The outline to draw this frame, if any.
    pub(crate) fn highlight(&self) -> Option<&sjk_ui::DrawList> {
        (!self.highlight.is_empty()).then_some(&self.highlight)
    }

    /// Save `note` about the confirmed selection into `directory`, returning the
    /// screenshot name to take (without extension; it lands in `screenshots`) and a
    /// line for the console.
    pub(crate) fn save(
        &mut self,
        note: &str,
        directory: &Path,
        screenshots: &Path,
    ) -> Option<(String, String)> {
        let selection = self.writing.take()?;
        let note = note.trim();
        if note.is_empty() {
            return None;
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs());
        let time = utc_text(seconds);
        let shot = format!("note_{seconds}");
        let screenshot = screenshots
            .join(format!("{shot}.jpg"))
            .display()
            .to_string();
        let json = selection.json(note, &time, &screenshot);
        let line = format!(
            "{time} | {} | {} | {} | {note}",
            selection.map,
            selection.setviewpos(),
            selection.summary()
        );
        let written = append(&directory.join("notes.jsonl"), &json.to_string())
            .and_then(|()| append(&directory.join("notes.txt"), &line));
        Some((
            shot,
            match written {
                Ok(()) => format!("note kept: {line}"),
                Err(error) => format!("^1note not kept in {}: {error}", directory.display()),
            },
        ))
    }
}

impl crate::GpuState {
    /// `inspect` with no card pinned and no player aimed at: select the world under the
    /// crosshair, or confirm the selection and open the note composer.
    pub(crate) fn world_note_press(&mut self) {
        let map = if self.resident.exploring() {
            self.resident.map.clone()
        } else {
            self.world_load_map.clone()
        };
        let press = self.world_notes.press(
            &self.bsp,
            &self.decal_surfaces,
            &mut self.trace_scratch,
            &map,
        );
        let centre = |text: String| (sjk_client::ServerEventKind::CenterPrint, text);
        let (kind, text) = match press {
            Press::Selected(summary) => {
                crate::log::progress(format_args!("note selection: {summary}"));
                centre(format!("{summary}\nPress inspect again to write a note"))
            }
            Press::Confirmed(subject) => {
                self.gameplay_input.release_keys();
                self.chat.open_note(subject);
                self.sync_cursor_policy();
                return;
            }
            Press::Nothing => centre("Nothing under the crosshair to note".to_owned()),
        };
        self.chat.receive(kind, text, None, Instant::now());
    }

    /// The note composer's Enter: keep the note with its selection and take the
    /// screenshot of the same view.
    pub(crate) fn save_world_note(&mut self, note: &str) {
        let Some(directory) = self
            .console
            .as_ref()
            .map(|console| console.config_directory().to_path_buf())
        else {
            return;
        };
        let Some((shot, line)) =
            self.world_notes
                .save(note, &directory, self.screenshots.directory())
        else {
            return;
        };
        self.screenshots
            .request(crate::screenshot::Request::jpeg_named(&shot));
        crate::log::progress(format_args!("{line}"));
        if let Some(console) = &mut self.console {
            console.push_log(line);
        }
    }
}

fn append(path: &PathBuf, line: &str) -> std::io::Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{line}")
}

/// `dd/mm/yyyy hh:mm:ss UTC` of Unix `seconds`.
fn utc_text(seconds: u64) -> String {
    let days = (seconds / 86_400) as i64;
    let rest = seconds % 86_400;
    // Howard Hinnant's civil_from_days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!(
        "{day:02}/{month:02}/{year} {:02}:{:02}:{:02} UTC",
        rest / 3_600,
        rest / 60 % 60,
        rest % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utc_dates_are_european() {
        assert_eq!(utc_text(0), "01/01/1970 00:00:00 UTC");
        // 07/10/2026 01:23:45 UTC.
        assert_eq!(utc_text(1_791_336_225), "07/10/2026 01:23:45 UTC");
    }

    #[test]
    fn boxes_have_twelve_edges_and_points_project_in_front_only() {
        let edges = box_edges(Vec3::ZERO, Vec3::ONE);
        assert_eq!(edges.len(), 12);
        assert!(edges.iter().all(|[a, b]| (*a - *b).length() == 1.0));
        let view = glam::camera::rh::view::look_at_mat4(Vec3::ZERO, Vec3::X, Vec3::Z);
        let projection =
            glam::camera::rh::proj::directx::perspective(1.2, 16.0 / 9.0, 2.0, 8_192.0);
        let view_projection = projection * view;
        let centre = project(
            &view_projection,
            [1920.0, 1080.0],
            Vec3::new(100.0, 0.0, 0.0),
        )
        .expect("in front");
        assert!((centre[0] - 960.0).abs() < 0.5 && (centre[1] - 540.0).abs() < 0.5);
        assert!(
            project(
                &view_projection,
                [1920.0, 1080.0],
                Vec3::new(-100.0, 0.0, 0.0)
            )
            .is_none()
        );
    }

    #[test]
    fn rays_enter_boxes_ahead_only() {
        let low = Vec3::new(10.0, -1.0, -1.0);
        let high = Vec3::new(12.0, 1.0, 1.0);
        assert_eq!(ray_box(Vec3::ZERO, Vec3::X, low, high), Some(10.0));
        assert_eq!(ray_box(Vec3::ZERO, -Vec3::X, low, high), None);
        assert_eq!(ray_box(Vec3::ZERO, Vec3::Y, low, high), None);
        // From inside, the box is met at once.
        assert_eq!(
            ray_box(Vec3::new(11.0, 0.0, 0.0), Vec3::X, low, high),
            Some(0.0)
        );
    }

    #[test]
    fn origins_parse_and_the_nearest_entity_is_named() {
        assert_eq!(
            parse_vector(Some("1 2.5 -3")),
            Some(Vec3::new(1.0, 2.5, -3.0))
        );
        assert_eq!(parse_vector(Some("1 2")), None);
        let entities = sjk_entity::parse_entity_lump(
            b"{\n\"classname\" \"worldspawn\"\n}\n\
              {\n\"classname\" \"light\"\n\"origin\" \"0 0 100\"\n}\n\
              {\n\"classname\" \"misc_model\"\n\"origin\" \"0 0 20\"\n}\n",
        )
        .expect("parses");
        assert_eq!(nearest_entity(&entities, Vec3::ZERO), Some(2));
        assert_eq!(
            nearest_entity(&entities, Vec3::new(0.0, 0.0, 1_000.0)),
            None
        );
    }

    #[test]
    fn a_note_needs_a_confirmed_selection_and_text() {
        let mut notes = Notes::default();
        let directory = std::env::temp_dir().join(format!("sjk-notes-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("directory");
        assert!(notes.save("hello", &directory, &directory).is_none());
        notes.writing = Some(Selection {
            map: "maps/mp/ffa3.bsp".into(),
            pose: "(0 0 0) : 90 0".into(),
            eye: Vec3::new(10.0, 20.0, 76.0),
            forward: Vec3::Y,
            hit: Vec3::new(10.0, 120.0, 76.0),
            normal: -Vec3::Y,
            distance: 100.0,
            surface: Some(Surface {
                index: 7,
                shader: "textures/imperial/basic_floor".into(),
                kind: "planar",
                lightmap: 2,
                vertices: 4,
                material: 3,
            }),
            entity: None,
            collision: None,
            edges: Vec::new(),
            selected: Instant::now(),
        });
        let (shot, line) = notes
            .save("  too shiny  ", &directory, &directory)
            .expect("saved");
        assert!(shot.starts_with("note_"));
        assert!(line.contains("setviewpos 10 20 40 90"), "{line}");
        assert!(line.contains("textures/imperial/basic_floor (surface 7, lightmapped)"));
        let json = std::fs::read_to_string(directory.join("notes.jsonl")).expect("jsonl");
        let value: serde_json::Value = serde_json::from_str(json.trim()).expect("json");
        assert_eq!(value["note"], "too shiny");
        assert_eq!(value["surface"]["shader"], "textures/imperial/basic_floor");
        let shot_path = directory.join(format!("{shot}.jpg")).display().to_string();
        assert_eq!(value["screenshot"], shot_path);
        // Saving consumes the selection.
        assert!(notes.save("again", &directory, &directory).is_none());
        let _ = std::fs::remove_dir_all(&directory);
    }
}
