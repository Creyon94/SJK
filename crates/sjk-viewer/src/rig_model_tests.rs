use super::*;
use sjk_model::GlaBone;
use sjk_model::gltf::{GltfJoint, GltfPrimitive};

fn identity_rows() -> [[f32; 4]; 3] {
    [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
    ]
}

fn translated(x: f32, y: f32, z: f32) -> [[f32; 4]; 3] {
    [[1.0, 0.0, 0.0, x], [0.0, 1.0, 0.0, y], [0.0, 0.0, 1.0, z]]
}

/// A driver with a pelvis 40 units up and a cranium above it.
fn driver() -> Gla {
    let bone = |name: &str, parent, base| GlaBone {
        name: name.to_owned(),
        flags: 0,
        parent,
        base_pose: base,
        inverse_base_pose: identity_rows(),
        children: Vec::new(),
    };
    Gla {
        name: "_humanoid".into(),
        scale: 1.0,
        bones: vec![
            bone("model_root", None, identity_rows()),
            bone("pelvis", Some(0), translated(0.0, 0.0, 40.0)),
            bone("cranium", Some(1), translated(0.0, 0.0, 60.0)),
        ],
        frames: Vec::new(),
        compressed_bones: Vec::new(),
    }
}

/// A rig with a pelvis one metre up (glTF +Y) and a head half a metre above it,
/// one vertex on the head.
fn rig() -> GltfModel {
    let joint = |name: &str, parent, y: f32| GltfJoint {
        name: name.to_owned(),
        parent,
        bind: translated(0.0, y, 0.0),
        inverse_bind: translated(0.0, -y, 0.0),
    };
    GltfModel {
        joints: vec![joint("pelvis", None, 1.0), joint("head", Some(0), 1.5)],
        primitives: vec![GltfPrimitive {
            base_color_image: Some("textures/skin.png".into()),
            positions: vec![[0.0, 1.6, 0.1]; 3],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            texture_coordinates: vec![[0.0; 2]; 3],
            joints: vec![[1, 0, 0, 0]; 3],
            weights: vec![[1.0, 0.0, 0.0, 0.0]; 3],
            indices: vec![0, 1, 2],
            ..Default::default()
        }],
    }
}

fn manifest(text: &str) -> Manifest {
    Manifest::parse(text).expect("the manifest")
}

#[test]
fn a_manifest_reads_every_key_and_rejects_nonsense() {
    let parsed = manifest(
        "// wolf\nmodel wolf.gltf\ndriver models/players/jedi_hm/\npreset Mannequin\nscale 40\nmap tail pelvis\nmap ear_l cranium as head\n",
    );
    assert_eq!(parsed.model.as_deref(), Some("wolf.gltf"));
    assert_eq!(parsed.driver, "models/players/jedi_hm");
    assert_eq!(parsed.scale, Some(40.0));
    assert_eq!(parsed.map[1].align, Align::As("head".into()));
    assert!(Manifest::parse("scale -1").is_err());
    assert!(Manifest::parse("colour red").is_err());
    assert_eq!(manifest("").driver, DEFAULT_DRIVER);
}

#[test]
fn manifest_map_lines_replace_preset_entries() {
    let map = manifest("map head cervical").bone_map().expect("the map");
    let head: Vec<_> = map.iter().filter(|e| e.joint == "head").collect();
    assert_eq!(head.len(), 1);
    assert_eq!(head[0].bone, "cervical");
    assert!(manifest("preset nothing").bone_map().is_err());
}

#[test]
fn auto_scale_fits_the_pelvis_and_the_axes_turn_to_z_up() {
    let rig = RigModel::fit(rig(), "models/players/wolf", &manifest(""), &driver()).unwrap();
    // 1 m pelvis onto a 40-unit pelvis: 40 units a metre; glTF +Y becomes +Z,
    // glTF +Z (front) becomes -Y.
    let vertex = rig.surfaces[0].vertices[0].position;
    assert!((vertex[2] - 64.0).abs() < 1e-4, "{vertex:?}");
    assert!((vertex[1] + 4.0).abs() < 1e-4, "{vertex:?}");
    assert_eq!(
        rig.surfaces[0].shader,
        "models/players/wolf/textures/skin.png"
    );
}

#[test]
fn the_base_pose_gives_the_bind_pose() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    rig.pose(&bones, &mut scratch);
    let palette = &scratch.palette;
    for matrix in palette {
        for (row, expected) in matrix.iter().zip(identity_rows()) {
            for (a, b) in row.iter().zip(expected) {
                assert!((a - b).abs() < 1e-4, "{palette:?}");
            }
        }
    }
}

#[test]
fn a_turned_driver_bone_turns_its_joint_and_children() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    // The cranium turned 90 degrees about Z around its base position (0, 0, 60).
    let turn = [
        [0.0, -1.0, 0.0, 60.0],
        [1.0, 0.0, 0.0, 60.0],
        [0.0, 0.0, 1.0, 0.0],
    ];
    let bones = vec![identity_rows(), identity_rows(), turn];
    let mut scratch = Scratch::default();
    rig.pose(&bones, &mut scratch);
    let vertex = Vec3A::from_array(rig.surfaces[0].vertices[0].position);
    let moved = affine(&scratch.palette[1]).transform_point3a(vertex);
    // The CPU path skins the same way.
    let (cpu, _) = skin(&rig.surfaces[0].vertices[0], &scratch.palette);
    assert!((Vec3A::from_array(cpu) - moved).length() < 1e-4);
    // Before: 4 units in front (-Y) of the head joint; after a quarter turn about +Z
    // the offset points along +X, and the head joint itself stays where it was.
    let head = rig.joints[1].bind.translation;
    let offset = moved - head;
    assert!(
        (offset.x - 4.0).abs() < 1e-3 && offset.y.abs() < 1e-3,
        "{offset:?}"
    );
}
