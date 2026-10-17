use super::*;
use crate::saber::Color;

#[path = "black_saber_gpu_test.rs"]
mod gpu_test;

fn game(server: &str, colors: &str) -> GameState {
    let mut game = GameState::empty_local(0);
    game.replace_config_string(0, server.as_bytes().to_vec())
        .unwrap();
    game.replace_config_string(1131, colors.as_bytes().to_vec())
        .unwrap();
    game
}

#[test]
fn plugin_three_is_allowed_when_its_disable_bit_is_clear() {
    let game = game(r"\gamename\JA+ Mod v2.5", r"\c1\11\c2\11");
    let orange = BladeColor::Retail(Color::Orange);
    let fallback = BladeColor::Retail(Color::Blue);
    for disabled in [0, 1536, 1 << 9, 1 << 10] {
        let allowed = Policy::new(Some(&game), true, disabled, 0);
        assert_eq!(allowed.color(1, 0, fallback), BladeColor::Black);
        assert_eq!(allowed.color(1, 1, fallback), BladeColor::Black);
        let denied = Policy::new(Some(&game), true, disabled | DISABLE_BIT, 0);
        assert_eq!(denied.color(1, 0, fallback), orange);
        assert_eq!(denied.color(1, 1, fallback), orange);
    }
    assert_eq!(
        Policy::new(Some(&game), false, 1536, 0).color(1, 0, fallback),
        orange
    );
}

#[test]
fn contracts_and_reconnects_do_not_leave_black_on_an_unsupported_server() {
    for server in [
        r"\gamename\JA+ Mod",
        r"\fs_game\japlus",
        r"\game\jaPRO",
        r"\gamename\TaystJK",
    ] {
        let supported = game(server, r"\c1\11");
        assert_eq!(
            Policy::new(Some(&supported), true, 0, 0).color(1, 0, BladeColor::Rgb([0; 3])),
            BladeColor::Black
        );
    }
    for server in [
        r"\gamename\basejka",
        r"\hostname\A JA+ fan server\gamename\basejka",
        "",
    ] {
        let unsupported = game(server, r"\c1\11");
        assert_eq!(
            Policy::new(Some(&unsupported), true, 0, 0).color(1, 0, BladeColor::Rgb([0; 3])),
            BladeColor::Retail(Color::Orange)
        );
    }
    assert_eq!(
        Policy::new(None, true, 0, 0).color(1, 0, BladeColor::Rgb([0; 3])),
        BladeColor::Rgb([0; 3])
    );
}

#[test]
fn indices_cosmetics_secondary_blades_and_muted_players_remain_distinct() {
    let mut game = game(r"\gamename\JA+ Mod", r"\c1\11santahat\c2\6\c4\0");
    game.replace_config_string(1194, br"\c1\6\c2\23cape".to_vec())
        .unwrap();
    let fallback = BladeColor::Rgb([0; 3]);
    let policy = Policy::new(Some(&game), true, 0, 0);
    assert_eq!(policy.color(1, 0, fallback), BladeColor::Black);
    assert_eq!(policy.color(1, 1, fallback), fallback);
    assert_eq!(policy.color(64, 0, fallback), fallback);
    assert_eq!(policy.color(64, 1, fallback), BladeColor::Black);
    for (entity, hand) in [(0, 0), (65, 0), (1, 2)] {
        assert_eq!(policy.color(entity, hand, fallback), fallback);
    }
    assert_eq!(
        Policy::new(Some(&game), true, 0, 1).color(1, 0, fallback),
        fallback
    );
    // A live colour change, including an invalid negative number, clears the bit.
    game.replace_config_string(1131, br"\c1\-1\c2\6".to_vec())
        .unwrap();
    assert_eq!(
        Policy::new(Some(&game), true, 0, 0).color(1, 0, fallback),
        fallback
    );
}

#[test]
fn black_material_is_separate_from_rgb_and_all_skin_slots() {
    assert_ne!(
        BladeColor::Black.material(),
        BladeColor::Rgb([0; 3]).material()
    );
    assert_eq!(
        BladeColor::from_material(super::super::BLACK_MATERIAL, [1.; 3]),
        BladeColor::Black
    );
    let blade = crate::saber::Blade {
        base: [0.; 3],
        direction: [0., 0., 1.],
        length: 40.,
        radius: 3.,
    };
    for instance in crate::saber::Instance::pair(blade, BladeColor::Black) {
        assert_eq!(instance.skin_index(), None);
    }
    let mut lights = crate::dynamic_lights::PointLightList::default();
    let mut blades = [None; 8];
    blades[0] = Some(blade);
    crate::saber_submission::lights::append(
        &mut lights,
        &blades,
        BladeColor::Black,
        1,
        false,
        1000,
        1,
    );
    assert!(lights.as_slice().is_empty());
    let source = include_str!("saber.wgsl");
    assert!(source.contains(&format!(
        "const BLACK_MATERIAL: u32 = {}u;",
        super::super::BLACK_MATERIAL
    )));
}

#[test]
fn generated_black_images_have_coverage_in_alpha_and_no_emission() {
    let vfs = sjk_vfs::VirtualFileSystem::new();
    let shaders = sjk_shader::ShaderCatalog::default();
    for kind in ["glow", "core", "trail"] {
        let image = image(&vfs, &shaders, kind);
        assert!(image.pixels().all(|pixel| pixel.0[..3] == [0; 3]));
        assert!(image.pixels().any(|pixel| pixel[3] > 0));
        assert!(image.pixels().any(|pixel| pixel[3] == 0));
    }
}

#[test]
fn installed_black_textures_keep_their_rgba_and_the_setting_defaults_on() {
    let texture = image::RgbaImage::from_pixel(2, 2, image::Rgba([80, 40, 20, 128]));
    let mut png = std::io::Cursor::new(Vec::new());
    texture.write_to(&mut png, image::ImageFormat::Png).unwrap();
    let mut vfs = sjk_vfs::VirtualFileSystem::new();
    vfs.mount_memory(
        "synthetic",
        [("gfx/effects/sabers/blackcore.png", png.into_inner())],
    )
    .unwrap();
    assert_eq!(
        image(&vfs, &sjk_shader::ShaderCatalog::default(), "core"),
        texture
    );
    let directory = tempfile::tempdir().unwrap();
    let mut console =
        crate::console::ViewerConsole::new(directory.path().join("config.cfg")).unwrap();
    assert_eq!(console.bool_cvar(CVAR), Some(true));
    assert!(console.set_cvar(CVAR, "0"));
    assert_eq!(console.bool_cvar(CVAR), Some(false));
}

#[test]
fn black_saber_shaders_validate_without_game_assets() {
    use wgpu::naga;
    for source in [include_str!("saber.wgsl"), include_str!("saber_trail.wgsl")] {
        let module = naga::front::wgsl::parse_str(source)
            .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::all(),
        )
        .validate(&module)
        .unwrap_or_else(|error| panic!("{}", error.emit_to_string(source)));
        assert!(
            module
                .entry_points
                .iter()
                .any(|entry| entry.name == "fragment_black")
        );
    }
}

#[test]
#[ignore = "release-only CPU cost measurement, not an FPS or GPU benchmark"]
fn black_saber_policy_release_cost() {
    let mut game = game(r"\gamename\JA+ Mod v2.5", r"\c1\11\c2\11");
    for client in 1..64 {
        game.replace_config_string(
            1131 + client,
            br"\n\Synthetic player\c1\11hat\c2\11cape\c3\0\c4\0".to_vec(),
        )
        .unwrap();
    }
    let start = std::time::Instant::now();
    for _ in 0..20_000 {
        std::hint::black_box(Policy::new(
            Some(std::hint::black_box(&game)),
            true,
            1536,
            0,
        ));
    }
    eprintln!(
        "64-slot black-saber policy: {:.3} us/frame (20,000 synthetic scans)",
        start.elapsed().as_secs_f64() * 1_000_000. / 20_000.
    );
}
