//! CG_PlayerFootsteps material banks (cg_main.c registration order).
/// Walk/run samples, four variants each, registered during world loading.
pub const PATHS: [[[&str; 4]; 2]; 11] = [
    [
        [
            "sound/player/footsteps/stone_step1.wav",
            "sound/player/footsteps/stone_step2.wav",
            "sound/player/footsteps/stone_step3.wav",
            "sound/player/footsteps/stone_step4.wav",
        ],
        [
            "sound/player/footsteps/stone_run1.wav",
            "sound/player/footsteps/stone_run2.wav",
            "sound/player/footsteps/stone_run3.wav",
            "sound/player/footsteps/stone_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/mud_walk1.wav",
            "sound/player/footsteps/mud_walk2.wav",
            "sound/player/footsteps/mud_walk3.wav",
            "sound/player/footsteps/mud_walk4.wav",
        ],
        [
            "sound/player/footsteps/mud_run1.wav",
            "sound/player/footsteps/mud_run2.wav",
            "sound/player/footsteps/mud_run3.wav",
            "sound/player/footsteps/mud_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/dirt_step1.wav",
            "sound/player/footsteps/dirt_step2.wav",
            "sound/player/footsteps/dirt_step3.wav",
            "sound/player/footsteps/dirt_step4.wav",
        ],
        [
            "sound/player/footsteps/dirt_run1.wav",
            "sound/player/footsteps/dirt_run2.wav",
            "sound/player/footsteps/dirt_run3.wav",
            "sound/player/footsteps/dirt_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/sand_walk1.wav",
            "sound/player/footsteps/sand_walk2.wav",
            "sound/player/footsteps/sand_walk3.wav",
            "sound/player/footsteps/sand_walk4.wav",
        ],
        [
            "sound/player/footsteps/sand_run1.wav",
            "sound/player/footsteps/sand_run2.wav",
            "sound/player/footsteps/sand_run3.wav",
            "sound/player/footsteps/sand_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/snow_step1.wav",
            "sound/player/footsteps/snow_step2.wav",
            "sound/player/footsteps/snow_step3.wav",
            "sound/player/footsteps/snow_step4.wav",
        ],
        [
            "sound/player/footsteps/snow_run1.wav",
            "sound/player/footsteps/snow_run2.wav",
            "sound/player/footsteps/snow_run3.wav",
            "sound/player/footsteps/snow_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/grass_step1.wav",
            "sound/player/footsteps/grass_step2.wav",
            "sound/player/footsteps/grass_step3.wav",
            "sound/player/footsteps/grass_step4.wav",
        ],
        [
            "sound/player/footsteps/grass_run1.wav",
            "sound/player/footsteps/grass_run2.wav",
            "sound/player/footsteps/grass_run3.wav",
            "sound/player/footsteps/grass_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/metal_step1.wav",
            "sound/player/footsteps/metal_step2.wav",
            "sound/player/footsteps/metal_step3.wav",
            "sound/player/footsteps/metal_step4.wav",
        ],
        [
            "sound/player/footsteps/metal_run1.wav",
            "sound/player/footsteps/metal_run2.wav",
            "sound/player/footsteps/metal_run3.wav",
            "sound/player/footsteps/metal_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/pipe_step1.wav",
            "sound/player/footsteps/pipe_step2.wav",
            "sound/player/footsteps/pipe_step3.wav",
            "sound/player/footsteps/pipe_step4.wav",
        ],
        [
            "sound/player/footsteps/pipe_run1.wav",
            "sound/player/footsteps/pipe_run2.wav",
            "sound/player/footsteps/pipe_run3.wav",
            "sound/player/footsteps/pipe_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/gravel_walk1.wav",
            "sound/player/footsteps/gravel_walk2.wav",
            "sound/player/footsteps/gravel_walk3.wav",
            "sound/player/footsteps/gravel_walk4.wav",
        ],
        [
            "sound/player/footsteps/gravel_run1.wav",
            "sound/player/footsteps/gravel_run2.wav",
            "sound/player/footsteps/gravel_run3.wav",
            "sound/player/footsteps/gravel_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/rug_step1.wav",
            "sound/player/footsteps/rug_step2.wav",
            "sound/player/footsteps/rug_step3.wav",
            "sound/player/footsteps/rug_step4.wav",
        ],
        [
            "sound/player/footsteps/rug_run1.wav",
            "sound/player/footsteps/rug_run2.wav",
            "sound/player/footsteps/rug_run3.wav",
            "sound/player/footsteps/rug_run4.wav",
        ],
    ],
    [
        [
            "sound/player/footsteps/wood_walk1.wav",
            "sound/player/footsteps/wood_walk2.wav",
            "sound/player/footsteps/wood_walk3.wav",
            "sound/player/footsteps/wood_walk4.wav",
        ],
        [
            "sound/player/footsteps/wood_run1.wav",
            "sound/player/footsteps/wood_run2.wav",
            "sound/player/footsteps/wood_run3.wav",
            "sound/player/footsteps/wood_run4.wav",
        ],
    ],
];
/// The sound family selected by codemp's ground trace surface material.
pub fn material(flags: u32) -> usize {
    match flags & 31 {
        17 => 1,
        7 => 2,
        8 => 3,
        14 => 4,
        5 | 6 => 5,
        3 => 6,
        4 => 7,
        9 => 8,
        21 | 22 | 24 | 25 | 27 => 9,
        1 | 2 => 10,
        _ => 0,
    }
}

/// The sound family of ground whose map gives it no material (surface flags with no
/// `MATERIAL_*` bits), guessed from its shader's name; `None` keeps the default
/// (stone). SJK's addition: retail maps leave much of their sand, snow and grass
/// untagged (`mp/siege_desert`'s `siege/siege2sand`), so codemp plays stone steps
/// there. Only the shader's own file name is read, so a folder named `desert` or
/// `snow` does not make its metal floors sand or snow.
pub fn material_from_name(shader: &str) -> Option<usize> {
    let name = shader
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(shader)
        .to_ascii_lowercase();
    let has = |word: &str| name.contains(word);
    if has("sand") && !has("sandstone") && !has("thousand") {
        Some(3)
    } else if has("snow") {
        Some(4)
    } else if has("grass") {
        Some(5)
    } else if has("gravel") {
        Some(8)
    } else if has("mud") {
        Some(1)
    } else if has("dirt") {
        Some(2)
    } else if has("carpet") {
        Some(9)
    } else if has("wood") || has("plank") {
        Some(10)
    } else {
        None
    }
}

/// NPC classes whose cgame footstep routine deliberately remains silent.
pub fn allowed_class(class: u8) -> bool {
    !matches!(
        class,
        1 | 4 | 7 | 8 | 10 | 16 | 30 | 32 | 34 | 35 | 39 | 41 | 42 | 45
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn untagged_ground_is_guessed_from_the_shader_file_name() {
        assert_eq!(material_from_name("textures/siege/siege2sand"), Some(3));
        assert_eq!(material_from_name("textures/desert/sandfloor2old"), Some(3));
        assert_eq!(material_from_name("textures/hoth/snow_ground"), Some(4));
        assert_eq!(material_from_name("textures/yavin/grass_rocks"), Some(5));
        assert_eq!(material_from_name("textures/a/WOOD_planks"), Some(10));
        // Stone that names sand, and folders, are not ground.
        assert_eq!(material_from_name("textures/desert/sandstone_wall"), None);
        assert_eq!(material_from_name("textures/desert/metal_floor"), None);
        assert_eq!(material_from_name("textures/snow/metal_floor"), None);
        // The family indices are the sound banks of PATHS.
        assert!(PATHS[3][0][0].contains("sand"));
        assert!(PATHS[4][0][0].contains("snow"));
        assert!(PATHS[5][0][0].contains("grass"));
        assert!(PATHS[10][0][0].contains("wood"));
    }
}
