//! Off-screen frames of a map, for reviewing what a camera sees without a
//! window: a [`GpuState`] built windowless renders into an image of its own
//! ([`GpuState::headless_frame`]) that is read back to a PNG in
//! `target/world-shots`. The tests here are ignored: they need a GPU adapter
//! and the installed game data named by `JKA_GAME_DATA`.

use crate::*;

/// Run `shots` on a thread with the stack a whole client needs (a test
/// thread's 2 MiB overflows building one).
pub(crate) fn on_big_stack(shots: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .name("world shots".to_owned())
        .stack_size(256 << 20)
        .spawn(shots)
        .expect("the world shot thread")
        .join()
        .expect("the world shots ran");
}

/// Where the shots are written.
pub(crate) fn directory() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/world-shots")
}

/// A windowless client on `map` (`maps/mp/duel6.bsp`) at `size`, with the
/// main menu up when `menu` is given; `None` without a GPU adapter.
pub(crate) fn open(
    map: &str,
    size: [u32; 2],
    menu: Option<menu::ClientMenu>,
    cvars: &[(&str, &str)],
) -> Option<(GpuState, tempfile::TempDir)> {
    let game_data = PathBuf::from(
        std::env::var_os("JKA_GAME_DATA").expect("JKA_GAME_DATA names the GameData directory"),
    );
    let directory = tempfile::tempdir().expect("a profile directory");
    let mut console =
        console::ViewerConsole::new(directory.path().join("config.cfg")).expect("a console");
    // A throwaway profile must not register an identity with the hub or look
    // for updates.
    console.set_cvar("cl_identity", "0");
    console.set_cvar("cl_autoUpdate", "0");
    for (name, value) in cvars {
        console.set_cvar(name, value);
    }
    let (bsp, vfs) = assets::load_bsp(&game_data, map).expect("the map");
    let scene = StaticWorld::build(&bsp, MeshBuildOptions::default().with_sky_surfaces())
        .expect("the map's meshes");
    let shaders = assets::load_shaders(&vfs);
    let (camera_origin, camera_yaw) = assets::initial_camera(&bsp).expect("a camera");
    let bounds = bsp.render().models()[0].clone();
    let input = GpuWorldInput {
        scene,
        bsp,
        vfs: Arc::new(vfs),
        shaders,
        world_minimums: bounds.minimums,
        world_maximums: bounds.maximums,
        camera_origin,
        camera_yaw,
        player_preview: None,
        live_session: None,
        demo_session: None,
        build_game_state: None,
        build_snapshot: None,
        console: Some(console),
        client_menu: menu,
        game_data,
        connect_timeline: None,
        game_fonts: false,
        completed_map_changes: 0,
    };
    let mut gpu = match pollster::block_on(GpuState::new_for_target(None, size, input)) {
        Ok(gpu) => gpu,
        Err(error) => {
            eprintln!("no windowless GPU state ({error}); skipped");
            return None;
        }
    };
    gpu.is_menu_world = true;
    let format = gpu.context.format;
    gpu.headless_frame = Some(gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("world shot"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &ui_target::surface_view_formats(format, gpu.context.ui_direct),
    }));
    Some((gpu, directory))
}

/// Render `frames` frames (letting what loads on workers arrive) and save the
/// last as `name`.png; returns its path.
pub(crate) fn shoot(gpu: &mut GpuState, frames: u32, name: &str) -> PathBuf {
    for _ in 0..frames {
        let _ = gpu.render(&mut None);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let texture = gpu.headless_frame.as_ref().expect("a world shot target");
    let image = read_back(&gpu.device, &gpu.queue, texture);
    std::fs::create_dir_all(directory()).expect("the shot directory");
    let path = directory().join(format!("{name}.png"));
    image.save(&path).expect("write the shot");
    path
}

/// Put the camera at `origin`, turned `yaw` degrees counter-clockwise from
/// +X and `pitch` degrees up (the backdrop's convention, not `viewpos`'s).
pub(crate) fn aim(gpu: &mut GpuState, origin: [f32; 3], yaw: f32, pitch: f32) {
    gpu.camera_position = Vec3::from_array(origin);
    gpu.camera_yaw = yaw.to_radians();
    gpu.camera_pitch = pitch.to_radians();
}

/// Render `frames` frames and return the image.
pub(crate) fn frame(gpu: &mut GpuState, frames: u32) -> image::RgbaImage {
    for _ in 0..frames {
        let _ = gpu.render(&mut None);
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
    let texture = gpu.headless_frame.as_ref().expect("a world shot target");
    read_back(&gpu.device, &gpu.queue, texture)
}

/// `images` side by side, `columns` to a row, each scaled to `cell` pixels
/// wide, saved as `name`.png: a contact sheet to review many views at once.
pub(crate) fn sheet(images: &[image::RgbaImage], columns: u32, cell: u32, name: &str) -> PathBuf {
    let first = images.first().expect("a shot");
    let cell_height = cell * first.height() / first.width();
    let rows = (images.len() as u32).div_ceil(columns);
    let mut out = image::RgbaImage::new(columns * cell, rows * cell_height);
    for (index, shot) in images.iter().enumerate() {
        let small = image::imageops::resize(
            shot,
            cell,
            cell_height,
            image::imageops::FilterType::Triangle,
        );
        let (x, y) = (index as u32 % columns, index as u32 / columns);
        image::imageops::overlay(
            &mut out,
            &small,
            i64::from(x * cell),
            i64::from(y * cell_height),
        );
    }
    std::fs::create_dir_all(directory()).expect("the shot directory");
    let path = directory().join(format!("{name}.png"));
    out.save(&path).expect("write the sheet");
    path
}

/// The yaw and pitch (degrees, the backdrop's convention) from `from` to `at`.
pub(crate) fn look(from: [f32; 3], at: [f32; 3]) -> (f32, f32) {
    let direction = (Vec3::from_array(at) - Vec3::from_array(from)).normalize_or_zero();
    (
        direction.y.atan2(direction.x).to_degrees(),
        direction.z.clamp(-1.0, 1.0).asin().to_degrees(),
    )
}

/// Render each camera of `views` (name, origin, a point it looks at) and save
/// them as contact sheet `name`, printing each one's index, numbers and
/// whether it starts inside a wall.
pub(crate) fn sweep(gpu: &mut GpuState, views: &[(&str, [f32; 3], [f32; 3])], name: &str) {
    let _ = frame(gpu, 30);
    let mut images = Vec::new();
    for (index, (label, origin, at)) in views.iter().enumerate() {
        let (yaw, pitch) = look(*origin, *at);
        aim(gpu, *origin, yaw, pitch);
        images.push(frame(gpu, 6));
        let solid = gpu.bsp.point_contents(*origin, 1) != 0;
        println!(
            "{index:2}: {label} {origin:?} yaw {yaw:.1} pitch {pitch:.1}{}",
            if solid { " IN A WALL" } else { "" }
        );
    }
    println!("{}", sheet(&images, 4, 480, name).display());
}

/// The texture's pixels.
fn read_back(
    device: &wgpu::Device,
    queue: &frame_queue::FrameQueue,
    texture: &wgpu::Texture,
) -> image::RgbaImage {
    let (width, height) = (texture.width(), texture.height());
    let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT)
        * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("world shot readback"),
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("world shot readback"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(height),
            },
        },
        texture.size(),
    );
    queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |result| {
        result.expect("map the readback")
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let data = slice.get_mapped_range().expect("the mapped readback");
    let mut pixels = Vec::with_capacity((width * height * 4) as usize);
    for line in data.chunks(row as usize).take(height as usize) {
        pixels.extend_from_slice(&line[..(width * 4) as usize]);
    }
    drop(data);
    buffer.unmap();
    image::RgbaImage::from_raw(width, height, pixels).expect("the shot's pixels")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The world from the duel6 camera the menu starts on, to check the
    /// harness itself.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_from_its_start() {
        on_big_stack(|| {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1280, 720], None, &[])
            else {
                return;
            };
            let path = shoot(&mut gpu, 40, "duel6-start");
            println!("{}", path.display());
        });
    }

    /// A plan of duel6 from above: every upward-facing surface coloured by its
    /// height (dark low, light high), a grid every 256 units (brighter every
    /// 1024, the axes brightest), spawns red and the intermission green. One
    /// pixel is 4 units, x to the right, y up; the image's centre is (0, 0).
    #[test]
    #[ignore = "reads the installed game data named by JKA_GAME_DATA"]
    fn duel6_plan() {
        let game_data = PathBuf::from(std::env::var_os("JKA_GAME_DATA").expect("JKA_GAME_DATA"));
        let (bsp, _) = assets::load_bsp(&game_data, "maps/mp/duel6.bsp").expect("the map");
        let scene = StaticWorld::build(&bsp, MeshBuildOptions::default()).expect("meshes");
        const UNITS: f32 = 4.0;
        const SIDE: u32 = 1600;
        let half = SIDE as f32 * 0.5;
        let pixel = |x: f32, y: f32| (half + x / UNITS, half - y / UNITS);
        let (mut low, mut high) = (f32::MAX, f32::MIN);
        for batch in scene.batches() {
            for vertex in &batch.vertices {
                low = low.min(vertex.position[2]);
                high = high.max(vertex.position[2]);
            }
        }
        let mut height = vec![f32::MIN; (SIDE * SIDE) as usize];
        let mut image = image::RgbaImage::from_pixel(SIDE, SIDE, image::Rgba([12, 16, 28, 255]));
        for batch in scene.batches() {
            for triangle in batch.indices.chunks_exact(3) {
                let [a, b, c] = [0, 1, 2].map(|i| batch.vertices[triangle[i] as usize].position);
                let normal = Vec3::from(b) - Vec3::from(a);
                let normal = normal
                    .cross(Vec3::from(c) - Vec3::from(a))
                    .normalize_or_zero();
                if normal.z < 0.6 {
                    continue;
                }
                let points = [a, b, c].map(|p| pixel(p[0], p[1]));
                let (x0, x1) = (
                    points
                        .iter()
                        .map(|p| p.0)
                        .fold(f32::MAX, f32::min)
                        .floor()
                        .max(0.0) as u32,
                    points
                        .iter()
                        .map(|p| p.0)
                        .fold(f32::MIN, f32::max)
                        .ceil()
                        .min(SIDE as f32 - 1.0) as u32,
                );
                let (y0, y1) = (
                    points
                        .iter()
                        .map(|p| p.1)
                        .fold(f32::MAX, f32::min)
                        .floor()
                        .max(0.0) as u32,
                    points
                        .iter()
                        .map(|p| p.1)
                        .fold(f32::MIN, f32::max)
                        .ceil()
                        .min(SIDE as f32 - 1.0) as u32,
                );
                let edge = |p: (f32, f32), q: (f32, f32), r: (f32, f32)| {
                    (q.0 - p.0) * (r.1 - p.1) - (q.1 - p.1) * (r.0 - p.0)
                };
                let area = edge(points[0], points[1], points[2]);
                if area.abs() < 1e-6 {
                    continue;
                }
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let p = (x as f32 + 0.5, y as f32 + 0.5);
                        let w = [
                            edge(points[1], points[2], p) / area,
                            edge(points[2], points[0], p) / area,
                            edge(points[0], points[1], p) / area,
                        ];
                        if w.iter().any(|w| *w < 0.0) {
                            continue;
                        }
                        let z = w[0] * a[2] + w[1] * b[2] + w[2] * c[2];
                        let slot = (y * SIDE + x) as usize;
                        if z > height[slot] {
                            height[slot] = z;
                            let t = ((z - low) / (high - low)).clamp(0.0, 1.0);
                            let shade = (40.0 + 200.0 * t) as u8;
                            image.put_pixel(
                                x,
                                y,
                                image::Rgba([shade, shade, (shade as f32 * 0.85) as u8, 255]),
                            );
                        }
                    }
                }
            }
        }
        for step in (-12..=12).map(|i| i as f32 * 256.0) {
            let strength = if step == 0.0 {
                160
            } else if step % 1024.0 == 0.0 {
                90
            } else {
                40
            };
            let (gx, _) = pixel(step, 0.0);
            let (_, gy) = pixel(0.0, step);
            for t in 0..SIDE {
                for (x, y) in [(gx as u32, t), (t, gy as u32)] {
                    if x < SIDE && y < SIDE {
                        let mut p = *image.get_pixel(x, y);
                        p.0[2] = p.0[2].saturating_add(strength);
                        image.put_pixel(x, y, p);
                    }
                }
            }
        }
        let entities = sjk_entity::parse_entity_lump(bsp.entities()).expect("entities");
        for entity in &entities {
            let colour = match entity.classname() {
                Some(class) if class.starts_with("info_player_intermission") => [60, 255, 90, 255],
                Some(class) if class.starts_with("info_player") => [255, 60, 60, 255],
                _ => continue,
            };
            let Ok(Some(origin)) = entity.vector("origin") else {
                continue;
            };
            let (cx, cy) = pixel(origin[0], origin[1]);
            for dy in -4..=4 {
                for dx in -4..=4 {
                    let (x, y) = (cx as i64 + dx, cy as i64 + dy);
                    if (0..SIDE as i64).contains(&x) && (0..SIDE as i64).contains(&y) {
                        image.put_pixel(x as u32, y as u32, image::Rgba(colour));
                    }
                }
            }
        }
        std::fs::create_dir_all(directory()).expect("the shot directory");
        let path = directory().join("duel6-plan.png");
        image.save(&path).expect("write the plan");
        println!("heights {low} to {high}; {}", path.display());
    }

    /// The menu's camera tour on duel6 as authored: each shot's start and end
    /// side by side, a row a shot, checked to start and end in open air with
    /// nothing solid between.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_tour() {
        on_big_stack(|| {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [960, 540], None, &[]) else {
                return;
            };
            let shots = menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
            let _ = frame(&mut gpu, 30);
            let mut images = Vec::new();
            for (index, shot) in shots.iter().enumerate() {
                for origin in [shot.from, shot.to] {
                    let (yaw, pitch) = look(origin, shot.at);
                    aim(&mut gpu, origin, yaw, pitch);
                    images.push(frame(&mut gpu, 6));
                }
                let trace = gpu.bsp.trace_box(
                    shot.from,
                    shot.to,
                    sjk_bsp::Aabb::new([-8.0; 3], [8.0; 3]).expect("a box"),
                    1,
                );
                println!(
                    "{index:2}: from {:?} to {:?} at {:?}{}",
                    shot.from,
                    shot.to,
                    shot.at,
                    if trace.fraction < 1.0 || trace.start_solid {
                        " BLOCKED"
                    } else {
                        ""
                    }
                );
            }
            println!("{}", sheet(&images, 4, 480, "duel6-tour").display());
        });
    }

    /// The SJK UI over the live duel6, as a player sees it: the main page on
    /// the tour's first shot, later shots of the tour (the menu's clock moved
    /// on), and Settings over the tour.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_sjk_menu() {
        on_big_stack(|| {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            // Past the first shot's fade-in.
            let _ = frame(&mut gpu, 20);
            gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
            shoot(&mut gpu, 4, "duel6-menu");
            for later in 1..3 {
                // To the next shot, then past its fade-in.
                gpu.ui_epoch -= std::time::Duration::from_millis(15_000);
                let _ = frame(&mut gpu, 1);
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                shoot(&mut gpu, 4, &format!("duel6-menu-{later}"));
            }
            if let (Some(menu), Some(console)) = (gpu.client_menu.as_mut(), gpu.console.as_ref()) {
                menu.open_sjk_settings(console, 7, crate::player_menu::ReturnTarget::MainMenu);
            }
            gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
            shoot(&mut gpu, 4, "duel6-menu-settings");
            // Key bindings, then one of them awaiting its key.
            for (capture, name) in [
                (false, "duel6-menu-keys"),
                (true, "duel6-menu-keys-capture"),
            ] {
                if let (Some(menu), Some(console)) =
                    (gpu.client_menu.as_mut(), gpu.console.as_ref())
                {
                    menu.sjk_keys_for_shot(console, "+attack", capture);
                }
                shoot(&mut gpu, 8, name);
            }
        });
    }

    /// The player screen on duel6's stage: the model standing where the
    /// route puts it, seen from the route's camera, in the style `style`.
    fn duel6_player(style: &'static str, name: &'static str) {
        on_big_stack(move || {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", style),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 4);
            if let (Some(menu), Some(console)) = (gpu.client_menu.as_mut(), gpu.console.as_ref()) {
                menu.open_player(console, crate::player_menu::ReturnTarget::MainMenu);
            }
            // Through the cut to the stage.
            let _ = frame(&mut gpu, 2);
            gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
            let path = shoot(&mut gpu, 6, name);
            println!("{}", path.display());
            // The Saber page (the saber thrown out to its shot), the Force page.
            for (page, row, suffix) in [(1, 2, "saber"), (2, 4, "force")] {
                if let Some(menu) = gpu.client_menu.as_mut() {
                    menu.player_page_for_shot(page, row);
                }
                let _ = frame(&mut gpu, 2);
                gpu.ui_epoch -= std::time::Duration::from_millis(3_000);
                let path = shoot(&mut gpu, 30, &format!("{name}-{suffix}"));
                println!("{}", path.display());
            }
        });
    }

    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_player_modern() {
        duel6_player("modern", "duel6-player-modern");
    }

    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_player_sjk() {
        duel6_player("sjk", "duel6-player-sjk");
    }

    /// Sol JK's pages in the SJK UI over the live duel6: What's new, Update
    /// (a newer release pretended out) and Identity (switched off: the shots'
    /// profile never registers).
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_sjk_pages() {
        on_big_stack(|| {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            crate::update::pretend_available("2026.1008.1");
            type Opens = fn(&mut console::ViewerConsole);
            let pages: [(&str, Opens); 3] = [
                (
                    "duel6-page-whats-new",
                    console::ViewerConsole::open_changelog,
                ),
                (
                    "duel6-page-update",
                    console::ViewerConsole::open_update_panel,
                ),
                (
                    "duel6-page-identity",
                    console::ViewerConsole::open_identity_panel,
                ),
            ];
            for (name, open_page) in pages {
                if let Some(console) = gpu.console.as_mut() {
                    open_page(console);
                }
                let path = shoot(&mut gpu, 6, name);
                println!("{}", path.display());
            }
        });
    }

    /// The SJK UI's server browser over the live duel6, on made-up servers:
    /// the list with the first server's map, numbers and players; a search
    /// with the left column's game type taking the keys; the password prompt.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_sjk_browser() {
        on_big_stack(|| {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            let _ = frame(&mut gpu, 10);
            if let Some(menu) = gpu.client_menu.as_mut() {
                menu.browser_for_shot();
            }
            // The chosen server's levelshot decodes on its worker.
            for _ in 0..240 {
                let _ = frame(&mut gpu, 1);
                if gpu
                    .client_menu
                    .as_ref()
                    .is_some_and(menu::ClientMenu::browser_picture_settled)
                {
                    break;
                }
            }
            gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
            println!("{}", shoot(&mut gpu, 4, "duel6-browser").display());
            if let Some(menu) = gpu.client_menu.as_mut() {
                menu.browser_search_for_shot("duel", 5);
            }
            println!("{}", shoot(&mut gpu, 8, "duel6-browser-search").display());
            if let Some(menu) = gpu.client_menu.as_mut() {
                menu.browser_search_for_shot("", 0);
                menu.browser_password_for_shot("saber");
            }
            println!("{}", shoot(&mut gpu, 4, "duel6-browser-password").display());
        });
    }

    /// The SJK UI's scoreboard over the live duel6, on made-up matches (no
    /// server): capture the flag with the classic menus and the look chosen on
    /// its own (`cg_scoreboardStyle sjk`, which loads the UI's families), then
    /// with the SJK UI's menus and the default `auto`: free for all, a full
    /// server, a duel and a power duel.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_sjk_scoreboard() {
        use crate::scoreboard::shot::Match;
        on_big_stack(|| {
            let cvars = [("ui_menuStyle", "classic"), ("cg_scoreboardStyle", "sjk")];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1920, 1080], None, &cvars)
            else {
                return;
            };
            // The menu tour's first view: down the west wing at the tower.
            let tour = menu_backdrop::tour_for("Yavin Training Grounds").expect("duel6's tour");
            let (yaw, pitch) = look(tour[0].from, tour[0].at);
            aim(&mut gpu, tour[0].from, yaw, pitch);
            let _ = frame(&mut gpu, 20);
            gpu.scoreboard.show_for_shot(Match::Capture);
            println!("{}", shoot(&mut gpu, 16, "duel6-scoreboard-ctf").display());
            if let Some(console) = gpu.console.as_mut() {
                console.set_cvar("ui_menuStyle", "sjk");
                console.set_cvar("cg_scoreboardStyle", "auto");
            }
            for (game, name) in [
                (Match::Free, "duel6-scoreboard-ffa"),
                (Match::Crowd, "duel6-scoreboard-full"),
                (Match::Duel, "duel6-scoreboard-duel"),
                (Match::PowerDuel, "duel6-scoreboard-power-duel"),
            ] {
                gpu.scoreboard.show_for_shot(game);
                println!("{}", shoot(&mut gpu, 16, name).display());
            }
            gpu.scoreboard.end_shot();
            drop(gpu);
            // A 4:3 window: the frame scales down to its width.
            let cvars = [("ui_menuStyle", "sjk")];
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [1440, 1080], None, &cvars)
            else {
                return;
            };
            aim(&mut gpu, tour[0].from, yaw, pitch);
            let _ = frame(&mut gpu, 20);
            gpu.scoreboard.show_for_shot(Match::Crowd);
            println!("{}", shoot(&mut gpu, 16, "duel6-scoreboard-4x3").display());
        });
    }

    /// The SJK UI's loading screen over the live duel6, on a made-up join of
    /// the JoF server: before the map is known (the tour behind), loading
    /// mp/ffa3 (its levelshot over the screen), and a failed join with and
    /// without the map known.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_sjk_loading() {
        use menu::classic::loading::{Stage, WorldStage};
        on_big_stack(|| {
            let menu = menu::ClientMenu::new(true, String::new());
            let cvars = [
                ("ui_menuStyle", "sjk"),
                (crate::settings::quick::HIDE_CVAR, "1"),
            ];
            let Some((mut gpu, _profile)) =
                open("maps/mp/duel6.bsp", [1920, 1080], Some(menu), &cvars)
            else {
                return;
            };
            // Past the tour's first fade-in.
            let _ = frame(&mut gpu, 20);
            gpu.ui_epoch -= std::time::Duration::from_millis(2_000);
            type State = (
                &'static str,
                Stage,
                bool,
                Option<WorldStage>,
                bool,
                Option<&'static str>,
            );
            let states: [State; 4] = [
                (
                    "duel6-loading-joining",
                    Stage::Challenging,
                    false,
                    None,
                    false,
                    None,
                ),
                (
                    "duel6-loading-map",
                    Stage::Loading,
                    true,
                    Some(WorldStage::Building),
                    true,
                    None,
                ),
                (
                    "duel6-loading-failed",
                    Stage::Loading,
                    true,
                    None,
                    false,
                    Some("server is full"),
                ),
                (
                    "duel6-loading-failed-early",
                    Stage::Connecting,
                    false,
                    None,
                    false,
                    Some("no answer from the server after 5 seconds"),
                ),
            ];
            for (name, stage, map, world, joined, error) in states {
                if let Some(menu) = gpu.client_menu.as_mut() {
                    menu.loading_for_shot(stage, map, world, joined, error);
                }
                // The levelshot decodes on its worker.
                for _ in 0..240 {
                    let _ = frame(&mut gpu, 1);
                    if gpu
                        .client_menu
                        .as_ref()
                        .is_some_and(menu::ClientMenu::loading_picture_settled)
                    {
                        break;
                    }
                }
                println!("{}", shoot(&mut gpu, 4, name).display());
            }
            // A server's change of map, its world (not the menu's) behind:
            // the navy ground instead, until the new map is named.
            gpu.is_menu_world = false;
            if let Some(menu) = gpu.client_menu.as_mut() {
                menu.loading_for_shot(Stage::Loading, true, None, true, None);
                menu.map_change_for_shot();
            }
            println!("{}", shoot(&mut gpu, 4, "duel6-loading-next-map").display());
        });
    }

    /// Hand-placed candidates for the menu's camera tour on duel6, for review.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_tour_candidates() {
        on_big_stack(|| {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [960, 540], None, &[]) else {
                return;
            };
            let views: &[(&str, [f32; 3], [f32; 3])] = &[
                (
                    "west wing (intermission)",
                    [-2224.0, 0.0, 888.0],
                    [-640.0, 0.0, 709.0],
                ),
                ("east wing", [2224.0, 0.0, 888.0], [640.0, 0.0, 709.0]),
                ("north wing", [0.0, 2224.0, 888.0], [0.0, 640.0, 709.0]),
                ("south wing", [0.0, -2224.0, 888.0], [0.0, -640.0, 709.0]),
                (
                    "high corner sw",
                    [-1100.0, -1100.0, 700.0],
                    [0.0, 0.0, 300.0],
                ),
                (
                    "high corner se",
                    [1100.0, -1100.0, 700.0],
                    [0.0, 0.0, 300.0],
                ),
                ("high corner ne", [1100.0, 1100.0, 700.0], [0.0, 0.0, 300.0]),
                (
                    "high corner nw",
                    [-1100.0, 1100.0, 700.0],
                    [0.0, 0.0, 300.0],
                ),
                (
                    "obelisk from below",
                    [-260.0, -260.0, 400.0],
                    [0.0, 0.0, 700.0],
                ),
                ("north bridge", [0.0, 1960.0, 760.0], [0.0, 0.0, 300.0]),
                (
                    "over the octagon wall",
                    [-560.0, 0.0, 700.0],
                    [0.0, 0.0, 300.0],
                ),
                (
                    "along the west wing",
                    [-2000.0, 600.0, 520.0],
                    [-2000.0, -600.0, 450.0],
                ),
                (
                    "quarter garden",
                    [-700.0, -1100.0, 460.0],
                    [-1400.0, -1400.0, 380.0],
                ),
                (
                    "south from the octagon",
                    [0.0, -1150.0, 330.0],
                    [0.0, -2000.0, 520.0],
                ),
                (
                    "down a diagonal arm",
                    [-380.0, -380.0, 470.0],
                    [-1000.0, -1000.0, 380.0],
                ),
                (
                    "east sunken court",
                    [1280.0, 200.0, 110.0],
                    [1280.0, -600.0, 160.0],
                ),
            ];
            sweep(&mut gpu, views, "duel6-tour-candidates");
        });
    }

    /// Every spawn and intermission point of duel6 at eye height, facing its
    /// own angle and its opposite, as a contact sheet: the raw material for
    /// the menu's camera tour.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn duel6_spawn_views() {
        on_big_stack(|| {
            let Some((mut gpu, _profile)) = open("maps/mp/duel6.bsp", [960, 540], None, &[]) else {
                return;
            };
            let entities = sjk_entity::parse_entity_lump(gpu.bsp.entities()).expect("entities");
            let mut views = Vec::new();
            for entity in &entities {
                let Some(class) = entity.classname() else {
                    continue;
                };
                if !class.starts_with("info_player") {
                    continue;
                }
                let Ok(Some(origin)) = entity.vector("origin") else {
                    continue;
                };
                let angle = entity.number("angle").ok().flatten().unwrap_or(0.0);
                for turn in [0.0, 180.0] {
                    views.push((class.to_owned(), origin, angle + turn));
                }
            }
            let _ = frame(&mut gpu, 30);
            let mut images = Vec::new();
            for (index, (class, origin, yaw)) in views.iter().enumerate() {
                aim(
                    &mut gpu,
                    [origin[0], origin[1], origin[2] + 40.0],
                    *yaw,
                    -4.0,
                );
                images.push(frame(&mut gpu, 6));
                println!("{index:2}: {class} {origin:?} yaw {yaw}");
            }
            println!("{}", sheet(&images, 4, 480, "duel6-spawns").display());
        });
    }
}
