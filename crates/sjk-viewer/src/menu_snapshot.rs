//! Menu snapshots without a window or a GPU: a menu screen's draw list and
//! text drawn into a PNG on the CPU, with the player's retail menu art, to
//! look at a layout (classic+ pages, `docs/classic-plus.md`) before anyone
//! plays it. It never opens a window or touches the network.
//!
//! The snapshots are an ignored test because they read the local game
//! installation:
//!
//! ```sh
//! JKA_GAME_DATA="/path/to/GameData" cargo test --release -p sjk-viewer \
//!     menu_snapshot -- --ignored --nocapture
//! ```
//!
//! They are written to `target/menu-snapshots/` in the workspace at
//! 1440x1080 (the 640x480 canvas at 2.25 units per pixel). The drawing is an
//! approximation of the UI renderer: flat rectangles without rounded corners,
//! nearest-texel art without its motion and flicker, the Inter font only, and
//! only the atlas icons a screen's test supplies. Text is drawn over every shape,
//! as on screen. The in-game menus are drawn over a retail levelshot standing for
//! the match.

use crate::keybind_editor::{Category, KeybindEditor};
use crate::menu::art::{ArtPiece, ArtSet};
use crate::menu::classic::layout::{Entry, Page, Panel, Span};
use crate::menu::classic::panel::{Frame, PanelFrame};
use crate::settings::SettingsMenu;
use image::{Rgba, RgbaImage};
use sjk_ui::{DrawCommand, DrawList, Rect};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

const VIEWPORT: [f32; 2] = [1440.0, 1080.0];

/// The retail menu art of the installation in `JKA_GAME_DATA`, decoded, and
/// the installation's files.
fn art() -> (ArtSet, Arc<sjk_vfs::VirtualFileSystem>) {
    let game = PathBuf::from(
        std::env::var_os("JKA_GAME_DATA").expect("set JKA_GAME_DATA to the GameData directory"),
    );
    let vfs = Arc::new(crate::assets::mount_game_data(&game).expect("mount the game data"));
    crate::menu::art::request(&vfs);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while crate::menu::art::decoded().is_none() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let decoded = crate::menu::art::decoded().expect("menu art decoded");
    let set = ArtPiece::ALL
        .into_iter()
        .filter(|piece| decoded.image(*piece).is_some())
        .fold(ArtSet::default(), |set, piece| set.with(piece));
    (set, vfs)
}

fn blend(image: &mut RgbaImage, x: i64, y: i64, color: [f32; 4]) {
    if x < 0 || y < 0 || x >= i64::from(image.width()) || y >= i64::from(image.height()) {
        return;
    }
    let pixel = image.get_pixel_mut(x as u32, y as u32);
    let alpha = color[3].clamp(0.0, 1.0);
    for (channel, source) in pixel.0.iter_mut().zip(color).take(3) {
        let below = f32::from(*channel) / 255.0;
        let value = source.clamp(0.0, 1.0) * alpha + below * (1.0 - alpha);
        *channel = (value * 255.0).round() as u8;
    }
}

/// Pixel bounds of `rect` inside `clip`.
fn span(rect: Rect, clip: Rect) -> (std::ops::Range<i64>, std::ops::Range<i64>) {
    let x0 = rect.x.max(clip.x).round() as i64;
    let y0 = rect.y.max(clip.y).round() as i64;
    let x1 = rect.right().min(clip.right()).round() as i64;
    let y1 = rect.bottom().min(clip.bottom()).round() as i64;
    (x0..x1, y0..y1)
}

fn fill(image: &mut RgbaImage, rect: Rect, clip: Rect, color: [f32; 4]) {
    let (xs, ys) = span(rect, clip);
    for y in ys {
        for x in xs.clone() {
            blend(image, x, y, color);
        }
    }
}

/// `source` mapped onto `rect` with texture coordinates `uv` at its corners
/// (top-left, top-right, bottom-right, bottom-left), tinted.
fn textured(
    image: &mut RgbaImage,
    rect: Rect,
    clip: Rect,
    source: &RgbaImage,
    uv: [[f32; 2]; 4],
    tint: [f32; 4],
    wrap: bool,
) {
    let (xs, ys) = span(rect, clip);
    let lerp =
        |a: [f32; 2], b: [f32; 2], t: f32| [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t];
    for y in ys {
        let t = ((y as f32 + 0.5) - rect.y) / rect.height;
        for x in xs.clone() {
            let s = ((x as f32 + 0.5) - rect.x) / rect.width;
            let [u, v] = lerp(lerp(uv[0], uv[1], s), lerp(uv[3], uv[2], s), t);
            let (u, v) = if wrap {
                (u.rem_euclid(1.0), v.rem_euclid(1.0))
            } else {
                (u.clamp(0.0, 1.0), v.clamp(0.0, 1.0))
            };
            let tx = ((u * source.width() as f32) as u32).min(source.width() - 1);
            let ty = ((v * source.height() as f32) as u32).min(source.height() - 1);
            let texel = source.get_pixel(tx, ty).0.map(|c| f32::from(c) / 255.0);
            let color = [0, 1, 2, 3].map(|c| texel[c] * tint[c]);
            blend(image, x, y, color);
        }
    }
}

/// Draw `list`'s shapes and art, then `vertices` (its text) from `atlas`.
fn raster(
    image: &mut RgbaImage,
    list: &DrawList,
    vertices: &[crate::text::TextVertex],
    atlas: &RgbaImage,
    icons: &HashMap<u32, RgbaImage>,
) {
    let decoded = crate::menu::art::decoded();
    let full = Rect::new(0.0, 0.0, image.width() as f32, image.height() as f32);
    let mut clips = vec![full];
    let mut opacity = vec![1.0_f32];
    let color = |c: sjk_ui::Color, o: f32| [c.r, c.g, c.b, c.a * o];
    let art = |texture: sjk_ui::TextureId| {
        let Some(piece) = ArtPiece::from_texture(texture) else {
            return icons.get(&texture.0).map(|icon| (icon, false));
        };
        Some((decoded?.image(piece)?, piece.wraps()))
    };
    for command in list.commands() {
        let clip = *clips.last().unwrap_or(&full);
        let o = *opacity.last().unwrap_or(&1.0);
        match command {
            DrawCommand::SolidRect { rect, color: c }
            | DrawCommand::RoundedRect { rect, color: c, .. } => {
                fill(image, *rect, clip, color(*c, o));
            }
            DrawCommand::GradientRect { rect, gradient, .. } => {
                const STEPS: usize = 32;
                for step in 0..STEPS {
                    let t = (step as f32 + 0.5) / STEPS as f32;
                    let (start, end) = (gradient.start, gradient.end);
                    let c = [
                        start.r + (end.r - start.r) * t,
                        start.g + (end.g - start.g) * t,
                        start.b + (end.b - start.b) * t,
                        (start.a + (end.a - start.a) * t) * o,
                    ];
                    let part = step as f32 / STEPS as f32;
                    let band = if gradient.vertical {
                        Rect::new(
                            rect.x,
                            rect.y + rect.height * part,
                            rect.width,
                            rect.height / STEPS as f32,
                        )
                    } else {
                        Rect::new(
                            rect.x + rect.width * part,
                            rect.y,
                            rect.width / STEPS as f32,
                            rect.height,
                        )
                    };
                    fill(image, band, clip, c);
                }
            }
            DrawCommand::Border {
                rect,
                width,
                color: c,
                ..
            } => {
                for edge in [
                    Rect::new(rect.x, rect.y, rect.width, *width),
                    Rect::new(rect.x, rect.bottom() - width, rect.width, *width),
                    Rect::new(rect.x, rect.y, *width, rect.height),
                    Rect::new(rect.right() - width, rect.y, *width, rect.height),
                ] {
                    fill(image, edge, clip, color(*c, o));
                }
            }
            DrawCommand::TexturedQuad {
                rect,
                texture,
                color: c,
            } => {
                if let Some((source, _)) = art(*texture) {
                    let uv = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
                    textured(image, *rect, clip, source, uv, color(*c, o), false);
                }
            }
            DrawCommand::TexturedQuadUv {
                rect,
                texture,
                color: c,
                uv,
            } => {
                if let Some((source, wraps)) = art(*texture) {
                    textured(image, *rect, full, source, *uv, color(*c, o), wraps);
                }
            }
            DrawCommand::PushClip(rect) => {
                let x = rect.x.max(clip.x);
                let y = rect.y.max(clip.y);
                clips.push(Rect::new(
                    x,
                    y,
                    (rect.right().min(clip.right()) - x).max(0.0),
                    (rect.bottom().min(clip.bottom()) - y).max(0.0),
                ));
            }
            DrawCommand::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            DrawCommand::PushOpacity(value) => opacity.push(o * value),
            DrawCommand::PopOpacity => {
                if opacity.len() > 1 {
                    opacity.pop();
                }
            }
            DrawCommand::Text { .. } => {}
        }
    }
    // Each glyph is six vertices `[x, y, u, v, r, g, b, a]` in clip space,
    // top-left first and bottom-right third; its coverage is the atlas
    // alpha averaged over each pixel's footprint.
    let floats: &[[f32; 8]] = bytemuck::cast_slice(vertices);
    let [width, height] = [image.width() as f32, image.height() as f32];
    let (atlas_width, atlas_height) = (atlas.width() as f32, atlas.height() as f32);
    for quad in floats.chunks_exact(6) {
        let pixel = |v: &[f32; 8]| [(v[0] + 1.0) * 0.5 * width, (1.0 - v[1]) * 0.5 * height];
        let [x0, y0] = pixel(&quad[0]);
        let [x1, y1] = pixel(&quad[2]);
        let (u0, v0, u1, v1) = (quad[0][2], quad[0][3], quad[2][2], quad[2][3]);
        let tint = [quad[0][4], quad[0][5], quad[0][6], quad[0][7]];
        for y in y0.floor() as i64..y1.ceil() as i64 {
            let texel_y = |py: f32| {
                let t = ((py - y0) / (y1 - y0)).clamp(0.0, 1.0);
                (v0 + (v1 - v0) * t) * atlas_height
            };
            let (ty0, ty1) = (
                texel_y(y as f32) as i64,
                texel_y(y as f32 + 1.0).ceil() as i64,
            );
            for x in x0.floor() as i64..x1.ceil() as i64 {
                let texel_x = |px: f32| {
                    let t = ((px - x0) / (x1 - x0)).clamp(0.0, 1.0);
                    (u0 + (u1 - u0) * t) * atlas_width
                };
                let (tx0, tx1) = (
                    texel_x(x as f32) as i64,
                    texel_x(x as f32 + 1.0).ceil() as i64,
                );
                let mut sum = 0.0_f32;
                let mut count = 0.0_f32;
                for ty in ty0..ty1.max(ty0 + 1) {
                    for tx in tx0..tx1.max(tx0 + 1) {
                        if (0..i64::from(atlas.width())).contains(&tx)
                            && (0..i64::from(atlas.height())).contains(&ty)
                        {
                            sum += f32::from(atlas.get_pixel(tx as u32, ty as u32).0[3]) / 255.0;
                        }
                        count += 1.0;
                    }
                }
                let coverage = sum / count.max(1.0);
                blend(image, x, y, [tint[0], tint[1], tint[2], tint[3] * coverage]);
            }
        }
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// What a snapshot is drawn with: the menu font, the UI atlas icons the
/// screen uses (by texture id), and the backdrop.
struct Snapshot {
    font: crate::text::FontAtlas,
    icons: HashMap<u32, RgbaImage>,
    /// A match under the in-game menus: the first retail levelshot found.
    in_match: Option<RgbaImage>,
}

impl Snapshot {
    /// Draw one screen over the menu backdrop (or the match) and write it as
    /// `name`.png.
    fn save(
        &self,
        name: &str,
        list: &DrawList,
        vertices: &[crate::text::TextVertex],
        over_match: bool,
    ) {
        let directory = workspace_root().join("target/menu-snapshots");
        std::fs::create_dir_all(&directory).expect("create the snapshot directory");
        let size = (VIEWPORT[0] as u32, VIEWPORT[1] as u32);
        let mut image = match (&self.in_match, over_match) {
            (Some(shot), true) => {
                image::imageops::resize(shot, size.0, size.1, image::imageops::FilterType::Triangle)
            }
            _ => RgbaImage::from_pixel(size.0, size.1, Rgba([18, 22, 30, 255])),
        };
        raster(&mut image, list, vertices, &self.font.image, &self.icons);
        let path = directory.join(format!("{name}.png"));
        image.save(&path).expect("write the snapshot");
        println!("{}", path.display());
    }
}

/// Image `path` of the game data, decoded.
fn decode(vfs: &sjk_vfs::VirtualFileSystem, path: &str) -> Option<RgbaImage> {
    let asset = vfs.read(path).ok().flatten()?;
    Some(crate::decode_image(&asset.bytes, path).ok()?.into_rgba8())
}

/// A retail levelshot to stand for the match behind the in-game menus.
fn match_backdrop(vfs: &sjk_vfs::VirtualFileSystem) -> Option<RgbaImage> {
    ["levelshots/mp/ffa3.jpg", "levelshots/mp/ffa5.jpg"]
        .into_iter()
        .find_map(|path| decode(vfs, path))
}

#[test]
#[ignore = "reads the installed game data named by JKA_GAME_DATA"]
fn menu_snapshot() {
    let (art, vfs) = art();
    let mut shots = Snapshot {
        font: crate::text::load_modern(1.0, None).expect("build the menu font"),
        icons: HashMap::new(),
        in_match: match_backdrop(&vfs),
    };
    let font = &shots.font;
    let directory = tempfile::tempdir().expect("scratch profile");
    let mut console =
        crate::console::ViewerConsole::new(directory.path().join("config.cfg")).expect("console");
    // A changed setting shows its mark and its default.
    console.set_cvar("r_hdrExposure", "1.5");
    // The classic option panels, focused on a row with something to say.
    let panels: [(&str, Page, Entry, Frame, &str); 9] = [
        (
            "setup-video-ingame",
            Page::Setup,
            Entry::Video,
            Frame::InGame,
            "cg_fov",
        ),
        (
            "setup-hud-ingame",
            Page::Setup,
            Entry::Hud,
            Frame::InGame,
            "cg_hudScale",
        ),
        (
            "setup-interface",
            Page::Setup,
            Entry::Interface,
            Frame::Main,
            "ui_menuStyle",
        ),
        (
            "setup-game-ingame",
            Page::Setup,
            Entry::GameOptions,
            Frame::InGame,
            "cg_saberTrail",
        ),
        (
            "setup-scoreboard",
            Page::Setup,
            Entry::Scoreboard,
            Frame::Main,
            "cg_showClientIDs",
        ),
        (
            "setup-sound-ingame",
            Page::Setup,
            Entry::Sound,
            Frame::InGame,
            "s_volume",
        ),
        (
            "renderer-image",
            Page::Renderer,
            Entry::RenderImage,
            Frame::Main,
            "r_hdrExposure",
        ),
        (
            "renderer-lighting-ingame",
            Page::Renderer,
            Entry::RenderLighting,
            Frame::InGame,
            "r_liveLighting",
        ),
        (
            "renderer-shadows",
            Page::Renderer,
            Entry::RenderShadows,
            Frame::Main,
            "r_sunShadowTaps",
        ),
    ];
    for (name, page, entry, frame, cvar) in panels {
        let mut menu = SettingsMenu::new();
        match entry.panel() {
            Some(Panel::Settings { caption, span }) => {
                let tab = SettingsMenu::tab_index(caption).expect("settings tab");
                menu.open_classic(&console, tab, span, frame);
            }
            Some(Panel::Renderer { tab }) => menu.open_classic_renderer(&console, tab, frame),
            Some(Panel::Group(group)) => menu.open_classic_group(&console, group, frame),
            other => panic!("{entry:?} has no settings panel: {other:?}"),
        }
        menu.select_cvar(cvar);
        let panel = PanelFrame {
            frame,
            page,
            active: entry,
            art,
        };
        let mut vertices = Vec::new();
        menu.append_classic(&mut vertices, &font.font, VIEWPORT, 1.0, &panel);
        shots.save(name, menu.draw_list(), &vertices, frame == Frame::InGame);
    }
    // Key-binding panels.
    let binds: [(&str, Entry, Category, Frame, &str); 4] = [
        (
            "controls-movement",
            Entry::Movement,
            Category::Movement,
            Frame::Main,
            "+moveup",
        ),
        (
            "controls-weapons-ingame",
            Entry::Weapons,
            Category::Weapons,
            Frame::InGame,
            "weapon 4",
        ),
        (
            "controls-force-ingame",
            Entry::ForcePowers,
            Category::Force,
            Frame::InGame,
            "+force_grip",
        ),
        (
            "controls-interaction",
            Entry::Interaction,
            Category::Interaction,
            Frame::Main,
            "use_bacta",
        ),
    ];
    for (name, entry, category, frame, command) in binds {
        let mut editor = KeybindEditor::new();
        for (texture, paths) in editor.snapshot_icons() {
            if let Some(image) = decode(&vfs, &paths[0]) {
                shots.icons.insert(texture.0, image);
            }
        }
        editor.open_classic(&console, category as usize, Span::ALL);
        editor.select_command(command);
        let panel = PanelFrame {
            frame,
            page: Page::Controls,
            active: entry,
            art,
        };
        let mut vertices = Vec::new();
        editor.append_classic(&mut vertices, &shots.font.font, VIEWPORT, 1.0, &panel);
        shots.save(name, editor.draw_list(), &vertices, frame == Frame::InGame);
    }
    in_game_menu(&shots, art);
    force_wheel(&mut shots, &vfs);
}

/// The classic in-game bar and its pop-ups over the match.
/// The HUD's Force wheel (JoF EJK's retail icon bar) over the match: JoF JA+'s
/// Repulse selected among real powers and the other JoF entries, then JA+ merc
/// mode's flamethrower in Lightning's place. Names draw in the menu font here.
fn force_wheel(shots: &mut Snapshot, vfs: &sjk_vfs::VirtualFileSystem) {
    use crate::hud::force_wheel::{ICONS, picture_names, snapshot};
    use sjk_client::force_wheel::{DASH, REPULSE, STASIS};
    let mut icons = [None; ICONS];
    for (slot, name) in picture_names() {
        let image = ["tga", "png", "jpg"]
            .into_iter()
            .find_map(|extension| decode(vfs, &format!("{name}.{extension}")));
        if let Some(image) = image {
            let id = sjk_ui::TextureId(crate::ui_renderer::FORCE_WHEEL_ICON_FIRST + slot as u32);
            shots.icons.insert(id.0, image);
            icons[slot] = Some(id);
        }
    }
    // Heal, Speed, Push, Pull, Mind Trick, Sense, Lightning, Grip, Drain.
    let powers = [0, 2, 3, 4, 5, 14, 7, 6, 13]
        .iter()
        .fold(0_u32, |bits, p| bits | 1 << p);
    let jof = (1 << STASIS) | (1 << REPULSE) | (1 << DASH);
    for (name, selected, flamethrower) in [
        ("hud-force-wheel-jof", REPULSE, false),
        ("hud-force-wheel-merc", 7, true),
    ] {
        let view = sjk_client::selection::SelectionView {
            inventory: false,
            available: powers | jof,
            selected,
            alpha: 1.0,
        };
        let (list, names) = snapshot(view, &icons, flamethrower, VIEWPORT);
        let mut vertices = Vec::new();
        crate::ui_renderer::append_text_commands(
            &list,
            names,
            &mut vertices,
            &shots.font.font,
            VIEWPORT,
            crate::text::TextStyle::NEUTRAL,
        );
        shots.save(name, &list, &vertices, true);
    }
}

fn in_game_menu(shots: &Snapshot, art: ArtSet) {
    use crate::ingame_menu::{InGameMenu, Page as Popup, View};
    let pages: [(&str, Popup, usize, bool); 5] = [
        ("ingame-bar", Popup::Main, 2, true),
        ("ingame-join", Popup::Team, 1, true),
        ("ingame-vote", Popup::Vote, 0, true),
        ("ingame-exit", Popup::Leave, 0, false),
        ("ingame-callvote", Popup::CallVote, 0, true),
    ];
    for (name, page, selected_row, team_game) in pages {
        let mut menu = InGameMenu::new();
        menu.set_style(crate::menu::style::MenuStyle::Classic, art);
        let view = View {
            page,
            selected_row,
            team: 1,
            team_game,
            siege: false,
            red_players: 3,
            blue_players: 2,
            vote_active: page == Popup::Vote,
            _frame: std::marker::PhantomData,
        };
        let mut vertices = Vec::new();
        menu.append(view, &mut vertices, &shots.font.font, VIEWPORT);
        shots.save(name, menu.draw_list(), &vertices, true);
    }
}
