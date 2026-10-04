use super::*;

#[test]
fn remap_key_matches_rd_vanilla_shader_names() {
    assert_eq!(
        remap_key("Textures\\Imperial\\Wall.TGA"),
        "textures/imperial/wall"
    );
    assert_eq!(remap_key("models/a.b/plain"), "models/a.b/plain");
    assert_eq!(remap_key("gfx/2d/crosshaira"), "gfx/2d/crosshaira");
    assert_eq!(remap_key("models/x/skin.v2"), "models/x/skin");
}

#[test]
fn remap_is_case_insensitive_and_ignores_extensions() {
    let mut remaps = ShaderRemaps::default();
    assert!(remaps.remap(
        RemapSource::Console,
        "Textures/Base/Wall",
        "textures/base/FLOOR.jpg",
        None,
    ));
    assert_eq!(
        remaps.target("textures/base/wall.tga"),
        Some("textures/base/floor")
    );
    assert_eq!(
        remaps.target("TEXTURES\\BASE\\WALL"),
        Some("textures/base/floor")
    );
    assert_eq!(remaps.target("textures/base/floor"), None);
    assert_eq!(remaps.resolve("textures/base/other"), "textures/base/other");
}

#[test]
fn remaps_are_one_level_deep() {
    let mut remaps = ShaderRemaps::default();
    remaps.remap(RemapSource::Console, "a", "b", None);
    remaps.remap(RemapSource::Console, "b", "c", None);
    // rd-vanilla draws `a` with `b`'s own stages; `b`'s remap is not followed.
    assert_eq!(remaps.target("a"), Some("b"));
    assert_eq!(remaps.target("b"), Some("c"));
}

#[test]
fn swapped_pair_draws_each_as_the_other() {
    // JoF EJK's crosshair a/j swap (`cg_main.c`) relies on this.
    let mut remaps = ShaderRemaps::default();
    remaps.remap(
        RemapSource::Console,
        "gfx/2d/crosshaira",
        "gfx/2d/crosshairj",
        None,
    );
    remaps.remap(
        RemapSource::Console,
        "gfx/2d/crosshairj",
        "gfx/2d/crosshaira",
        None,
    );
    assert_eq!(
        remaps.target("gfx/2d/crosshaira"),
        Some("gfx/2d/crosshairj")
    );
    assert_eq!(
        remaps.target("gfx/2d/crosshairj"),
        Some("gfx/2d/crosshaira")
    );
}

#[test]
fn remap_to_itself_clears_the_remap() {
    let mut remaps = ShaderRemaps::default();
    remaps.remap(RemapSource::Console, "a", "b", None);
    let generation = remaps.generation();
    assert!(remaps.remap(RemapSource::Console, "A.tga", "a", None));
    assert_eq!(remaps.target("a"), None);
    assert!(remaps.is_empty());
    assert!(remaps.generation() > generation);
    // A self-remap hides another source's remap too: the latest one wins.
    remaps.remap(RemapSource::Server, "a", "b", None);
    assert_eq!(remaps.target("a"), Some("b"));
    remaps.remap(RemapSource::Console, "a", "a", None);
    assert_eq!(remaps.target("a"), None);
    assert!(remaps.entries().iter().all(|entry| !entry.active));
}

#[test]
fn latest_remap_wins_across_sources_and_clearing_reveals_the_earlier_one() {
    let mut remaps = ShaderRemaps::default();
    remaps.remap(RemapSource::Map, "a", "map", None);
    remaps.remap(RemapSource::Server, "a", "server", None);
    assert_eq!(remaps.target("a"), Some("server"));
    remaps.remap(RemapSource::Console, "a", "console", None);
    assert_eq!(remaps.target("a"), Some("console"));
    // A repeated server update re-applies its entries over the console remap,
    // as `CG_ShaderStateChanged` calls R_RemapShader for every entry again.
    remaps.remap(RemapSource::Server, "a", "server", None);
    assert_eq!(remaps.target("a"), Some("server"));
    assert!(remaps.clear_source(RemapSource::Server));
    assert_eq!(remaps.target("a"), Some("console"));
    assert!(remaps.clear_source(RemapSource::Console));
    assert_eq!(remaps.target("a"), Some("map"));
    assert!(remaps.clear());
    assert!(remaps.is_empty());
    assert!(!remaps.clear());
}

#[test]
fn unchanged_tables_keep_their_generation() {
    let mut remaps = ShaderRemaps::default();
    assert!(remaps.remap(RemapSource::Console, "a", "b", None));
    let generation = remaps.generation();
    assert!(!remaps.remap(RemapSource::Console, "a", "b", None));
    assert!(!remaps.clear_source(RemapSource::Server));
    assert!(!remaps.set_level(RemapLevel::All));
    assert_eq!(remaps.generation(), generation);
}

#[test]
fn cg_remaps_levels_follow_eternaljk() {
    assert_eq!(RemapLevel::from_cvar(0), RemapLevel::Off);
    assert_eq!(RemapLevel::from_cvar(1), RemapLevel::MapOnly);
    assert_eq!(RemapLevel::from_cvar(2), RemapLevel::All);
    assert_eq!(RemapLevel::from_cvar(7), RemapLevel::All);
    assert_eq!(RemapLevel::from_cvar(-1), RemapLevel::All);
    assert_eq!(RemapLevel::default(), RemapLevel::All);

    let player = "Models/Players/kyle/body";
    let wall = "textures/imperial/wall";
    assert!(RemapLevel::All.allows(RemapSource::Server, player));
    assert!(RemapLevel::MapOnly.allows(RemapSource::Server, wall));
    assert!(!RemapLevel::MapOnly.allows(RemapSource::Server, player));
    assert!(!RemapLevel::MapOnly.allows(RemapSource::Server, "models\\players\\kyle\\body"));
    assert!(RemapLevel::MapOnly.allows(RemapSource::Server, "models/players"));
    assert!(!RemapLevel::Off.allows(RemapSource::Server, wall));
    // Only server-sent remaps are gated: the console command and the map's own
    // worldspawn keys apply at every level.
    for level in [RemapLevel::Off, RemapLevel::MapOnly, RemapLevel::All] {
        assert!(level.allows(RemapSource::Console, player));
        assert!(level.allows(RemapSource::Map, player));
    }
}

#[test]
fn changing_the_level_applies_live() {
    let mut remaps = ShaderRemaps::new(RemapLevel::All);
    remaps.remap(RemapSource::Server, "textures/a", "textures/b", Some(3.0));
    remaps.remap(RemapSource::Server, "models/players/kyle/body", "x", None);
    remaps.remap(RemapSource::Console, "textures/c", "textures/d", None);
    assert_eq!(remaps.target("textures/a"), Some("textures/b"));
    assert_eq!(remaps.time_offset("textures/b"), Some(3.0));

    assert!(remaps.set_level(RemapLevel::MapOnly));
    assert_eq!(remaps.target("textures/a"), Some("textures/b"));
    assert_eq!(remaps.target("models/players/kyle/body"), None);

    assert!(remaps.set_level(RemapLevel::Off));
    assert_eq!(remaps.target("textures/a"), None);
    assert_eq!(remaps.time_offset("textures/b"), None);
    assert_eq!(remaps.target("textures/c"), Some("textures/d"));
    let entries = remaps.entries();
    assert_eq!(entries.len(), 3);
    assert!(!entries[0].active && !entries[1].active && entries[2].active);

    assert!(remaps.set_level(RemapLevel::All));
    assert_eq!(remaps.target("models/players/kyle/body"), Some("x"));
}

#[test]
fn time_offsets_belong_to_the_target_and_survive_offsetless_remaps() {
    let mut remaps = ShaderRemaps::default();
    remaps.remap(RemapSource::Server, "a", "b", Some(12.5));
    assert_eq!(remaps.time_offset("B.tga"), Some(12.5));
    assert_eq!(remaps.time_offset("a"), None);
    // The console command passes no offset: rd-vanilla leaves the target's alone.
    remaps.remap(RemapSource::Console, "c", "b", None);
    assert_eq!(remaps.time_offset("b"), Some(12.5));
    remaps.remap(RemapSource::Server, "d", "b", Some(20.0));
    assert_eq!(remaps.time_offset("b"), Some(20.0));
}

#[test]
fn shader_state_parses_the_game_module_format() {
    // `BuildShaderStateConfig` writes `%s=%s:%5.2f@` per remap.
    let entries =
        parse_shader_state("textures/a/light=textures/a/light_off: 5.20@Models/B=models/c:123.45@");
    assert_eq!(
        entries,
        vec![
            ShaderStateEntry {
                old: "textures/a/light".into(),
                new: "textures/a/light_off".into(),
                time_offset: 5.2,
            },
            ShaderStateEntry {
                old: "Models/B".into(),
                new: "models/c".into(),
                time_offset: 123.45,
            },
        ]
    );
    assert!(parse_shader_state("").is_empty());
}

#[test]
fn shader_state_stops_at_a_malformed_entry() {
    // No closing `@`: CG_ShaderStateChanged leaves the loop without applying it.
    let entries = parse_shader_state("a=b:1.00@c=d:2.00");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].old, "a");
    assert!(!parse_shader_state("a=b@c=d:1@").is_empty());
    assert!(parse_shader_state("nothing here").is_empty());
    assert!(parse_shader_state("a=b").is_empty());
    let entries = parse_shader_state("a=b:junk@");
    assert_eq!(entries[0].time_offset, 0.0);
}

#[test]
fn shader_state_searches_like_strstr() {
    // strstr(n, ":") runs past the `@`, so a missing `:` swallows the next entry.
    let entries = parse_shader_state("a=b@c=d:1.50@");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].old, "a");
    assert_eq!(entries[0].new, "b@c=d");
    assert_eq!(entries[0].time_offset, 1.5);
}

#[test]
fn atof_matches_the_c_library() {
    assert_eq!(atof(" 5.20"), 5.2);
    assert_eq!(atof("\t-3"), -3.0);
    assert_eq!(atof("1e2x"), 100.0);
    assert_eq!(atof("1e"), 1.0);
    assert_eq!(atof(".5"), 0.5);
    assert_eq!(atof("7."), 7.0);
    assert_eq!(atof("."), 0.0);
    assert_eq!(atof("abc"), 0.0);
    assert_eq!(atof(""), 0.0);
}

#[test]
fn worldspawn_keys_follow_r_load_entities() {
    let fields = [
        ("classname", "worldspawn"),
        ("remapshader", "textures/a;textures/b"),
        ("remapshader2", "textures/c;textures/d;e"),
        ("RemapShader", "textures/x;textures/y"),
        ("vertexremapshader", "textures/v;textures/w"),
    ];
    assert_eq!(
        worldspawn_remaps(fields),
        vec![("textures/a", "textures/b"), ("textures/c", "textures/d;e")]
    );
    let broken = [
        ("remapshader", "textures/a;textures/b"),
        ("remapshader1", "no-separator"),
        ("remapshader2", "textures/c;textures/d"),
    ];
    assert_eq!(
        worldspawn_remaps(broken),
        vec![("textures/a", "textures/b")]
    );
}

#[test]
fn entries_list_in_application_order() {
    let mut remaps = ShaderRemaps::default();
    remaps.remap(RemapSource::Console, "a", "b", None);
    remaps.remap(RemapSource::Server, "c", "d", Some(1.0));
    remaps.remap(RemapSource::Console, "a", "e", None);
    let entries = remaps.entries();
    assert_eq!(entries.len(), 2);
    assert_eq!((entries[0].old, entries[0].new), ("c", "d"));
    assert_eq!(entries[0].source, RemapSource::Server);
    assert_eq!(entries[0].time_offset, Some(1.0));
    assert_eq!((entries[1].old, entries[1].new), ("a", "e"));
    assert!(entries.iter().all(|entry| entry.active));
}
