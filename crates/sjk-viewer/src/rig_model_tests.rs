use super::*;
use sjk_model::GlaBone;
use sjk_model::gltf::{GltfAnimation, GltfChannel, GltfJoint, GltfPath, GltfPrimitive};

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
/// one vertex on the head, and a clip "Nod" turning the head a quarter turn about
/// glTF +Y (up) over one second.
fn rig() -> GltfModel {
    let joint = |name: &str, parent, y: f32, rest: f32| GltfJoint {
        name: name.to_owned(),
        parent,
        bind: translated(0.0, y, 0.0),
        inverse_bind: translated(0.0, -y, 0.0),
        rest: translated(0.0, rest, 0.0),
    };
    let half = std::f32::consts::FRAC_1_SQRT_2;
    GltfModel {
        animations: vec![GltfAnimation {
            name: "Nod".into(),
            channels: vec![GltfChannel {
                joint: 1,
                path: GltfPath::Rotation,
                times: vec![0.0, 1.0],
                values: vec![[0.0, 0.0, 0.0, 1.0], [0.0, half, 0.0, half]],
                step: false,
            }],
        }],
        joints: vec![
            joint("pelvis", None, 1.0, 1.0),
            joint("head", Some(0), 1.5, 0.5),
        ],
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
    rig.pose(&bones, None, &mut scratch);
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
    rig.pose(&bones, None, &mut scratch);
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

#[test]
fn triangles_turn_to_the_game_winding() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    // glTF fronts are counter-clockwise; the game's `cull front` keeps clockwise ones.
    assert_eq!(rig.surfaces[0].indices, vec![0, 2, 1]);
}

#[test]
fn a_bolt_moves_from_the_driver_bone_onto_the_rig_joint() {
    let mut driver = driver();
    driver.bones.push(GlaBone {
        name: "ceyebrow".into(),
        flags: 0,
        parent: Some(2),
        base_pose: translated(0.0, -3.0, 62.0),
        inverse_base_pose: identity_rows(),
        children: Vec::new(),
    });
    // 50 units a metre: the rig's head joint sits at 75, the driver's cranium at 60.
    let rig = RigModel::fit(rig(), "", &manifest("scale 50"), &driver).unwrap();
    // The cranium shifted 10 units along +X; its bolt 5 in front and 5 above it.
    let bones = vec![
        identity_rows(),
        identity_rows(),
        translated(10.0, 0.0, 0.0),
        translated(10.0, 0.0, 0.0),
    ];
    let bolt = translated(10.0, -5.0, 65.0);
    let moved = rig.retarget_bolt(2, &bones, None, bolt);
    let origin = Vec3A::new(moved[0][3], moved[1][3], moved[2][3]);
    assert!(
        (origin - Vec3A::new(0.0, -5.0, 80.0)).length() < 1e-4,
        "{origin:?}"
    );
    // The orientation is the driver's.
    assert_eq!(moved[0][..3], bolt[0][..3]);
    // An unmapped bone (the eyebrow) follows its mapped parent's joint.
    let point = rig.retarget_point(3, &bones, None, Vec3A::new(10.0, -5.0, 65.0));
    assert!((point - origin).length() < 1e-4, "{point:?}");
}

/// The head vertex skinned with `scratch`'s palette, minus the head joint.
fn head_offset(rig: &RigModel, scratch: &Scratch) -> Vec3A {
    let vertex = Vec3A::from_array(rig.surfaces[0].vertices[0].position);
    affine(&scratch.palette[1]).transform_point3a(vertex) - rig.joints[1].bind.translation
}

#[test]
fn a_clip_plays_on_the_rigs_own_skeleton() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    assert_eq!(rig.clips.len(), 1);
    assert_eq!(rig.clips[0].duration, 1.0);
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    // At its first key the clip is the rest pose: the vertex stays 4 units in front
    // (-Y) and 4 above the head joint.
    let play = rig.play("Nod", 0.0, None, false).map(|play| ClipPlay {
        weight: 1.0,
        ..play
    });
    rig.pose(&bones, play, &mut scratch);
    let offset = head_offset(&rig, &scratch);
    assert!(
        (offset - Vec3A::new(0.0, -4.0, 4.0)).length() < 1e-4,
        "{offset:?}"
    );
    // At its end, a quarter turn about glTF +Y is one about the game's +Z.
    let play = rig.play("Nod", 1.0, None, false);
    assert_eq!(play.map(|play| play.weight), Some(1.0));
    rig.pose(&bones, play, &mut scratch);
    let offset = head_offset(&rig, &scratch);
    assert!(
        (offset - Vec3A::new(4.0, 0.0, 4.0)).length() < 1e-3,
        "{offset:?}"
    );
    // Half faded in, half way round.
    let half = ClipPlay {
        weight: 0.5,
        ..play.unwrap()
    };
    rig.pose(&bones, Some(half), &mut scratch);
    let offset = head_offset(&rig, &scratch);
    let side = 4.0 * std::f32::consts::FRAC_1_SQRT_2;
    assert!(
        (offset - Vec3A::new(side, -side, 4.0)).length() < 1e-3,
        "{offset:?}"
    );
    assert!(rig.play("Wave", 0.0, None, false).is_none());
}

#[test]
fn a_clip_fades_in_and_out_and_a_loop_wraps() {
    let play = ClipPlay::new(0, 2.0, 0.1, Some(5.0), false);
    assert!((play.weight - 0.4).abs() < 1e-6 && play.time == 0.1);
    let play = ClipPlay::new(0, 2.0, 3.0, Some(0.05), false);
    assert!((play.weight - 0.2).abs() < 1e-6 && play.time == 2.0);
    let play = ClipPlay::new(0, 2.0, 5.5, None, true);
    assert!(play.weight == 1.0 && (play.time - 1.5).abs() < 1e-6);
}

#[test]
fn a_bolt_follows_the_joint_a_clip_moves() {
    let rig = RigModel::fit(rig(), "", &manifest(""), &driver()).unwrap();
    let bones = vec![identity_rows(); 3];
    let mut scratch = Scratch::default();
    let bolt = translated(0.0, -5.0, 65.0);
    rig.pose(&bones, None, &mut scratch);
    let still = rig.retarget_bolt(2, &bones, Some(&scratch), bolt);
    assert_eq!(still, rig.retarget_bolt(2, &bones, None, bolt));
    rig.pose(&bones, rig.play("Nod", 1.0, None, false), &mut scratch);
    let turned = rig.retarget_bolt(2, &bones, Some(&scratch), bolt);
    let origin = Vec3A::new(turned[0][3], turned[1][3], turned[2][3]);
    assert!(
        (origin - Vec3A::new(5.0, 0.0, 65.0)).length() < 1e-3,
        "{origin:?}"
    );
    // Its axes turn with the head: X now points along +Y.
    assert!((turned[1][0] - 1.0).abs() < 1e-3, "{turned:?}");
}
