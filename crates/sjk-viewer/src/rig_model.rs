//! Rig models: a player model drawn from a glTF file with its own skeleton, posed
//! every frame from the stock `_humanoid` skeleton that still drives the player.
//!
//! A model folder holding [`MANIFEST`] is a rig model. The player keeps a stock
//! driver model (Kyle unless the manifest names another): its GLM and skeleton
//! carry the server's animations, bolts, saber traces and dismemberment exactly as
//! before, but are not drawn. The glTF mesh is drawn in their place, skinned with a
//! palette computed here from the driver's evaluated bone matrices: each mapped
//! joint takes its JKA bone's model-space rotation from the base pose, after its own
//! bind pose is turned onto the JKA bone's direction; unmapped joints (twist bones,
//! face, cloth) keep their bind pose under their parent. Which joint follows which
//! JKA bone is a bone map: a named preset from [`PRESETS`] plus `map` lines.
//!
//! See `docs/rig-models.md` for the file format.

use glam::{Affine3A, Mat3, Mat3A, Vec3, Vec3A};
use sjk_model::Gla;
use sjk_model::gltf::{GltfModel, parse};
use sjk_vfs::VirtualFileSystem;
use std::error::Error;

/// The file that makes a model folder a rig model.
pub(crate) const MANIFEST: &str = "sjk_rig.cfg";

/// The driver model when the manifest does not name one.
const DEFAULT_DRIVER: &str = "models/players/kyle";

/// A surface's shader when its material has no base colour image.
const UNTEXTURED_SHADER: &str = "*white";

/// One bone-map line: the rig joint, the JKA bone it follows, and how its bind pose
/// is aligned onto the JKA base pose.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MapEntry {
    pub(crate) joint: String,
    pub(crate) bone: String,
    pub(crate) align: Align,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Align {
    /// Keep the bind pose as it is (the head, the feet: no child to aim along).
    Rest,
    /// Turn the joint so its direction to `joint_end` matches the JKA bone's
    /// direction to `bone_end`.
    Toward { joint_end: String, bone_end: String },
    /// Use the alignment computed for another mapped joint (finger chains).
    As(String),
}

/// A named bone map and the function that builds it.
pub(crate) type Preset = (&'static str, fn() -> Vec<MapEntry>);

/// Built-in bone maps, by the skeleton naming they read. Supporting another naming
/// is a new entry here (or `map` lines in a model's manifest), never code elsewhere.
pub(crate) const PRESETS: &[Preset] = &[
    // Unreal Engine mannequin names (pelvis, spine_01..05, upperarm_l, thumb_01_l...),
    // the skeleton most game and community character rigs export. Checked on
    // 14/10/2026 with a 177-joint mannequin character in Blender against posed Kyle.
    ("mannequin", mannequin),
];

fn entry(joint: &str, bone: &str, align: Align) -> MapEntry {
    MapEntry {
        joint: joint.to_owned(),
        bone: bone.to_owned(),
        align,
    }
}

fn toward(joint_end: &str, bone_end: &str) -> Align {
    Align::Toward {
        joint_end: joint_end.to_owned(),
        bone_end: bone_end.to_owned(),
    }
}

/// The Unreal mannequin onto `_humanoid`. Five spine joints share JKA's three; JKA's
/// two-joint finger groups (thumb d1, index/middle d2, ring/pinky d4) drive all
/// three joints of each finger.
fn mannequin() -> Vec<MapEntry> {
    let mut map = vec![
        entry("pelvis", "pelvis", toward("spine_01", "lower_lumbar")),
        entry(
            "spine_01",
            "lower_lumbar",
            toward("spine_03", "upper_lumbar"),
        ),
        entry(
            "spine_02",
            "lower_lumbar",
            toward("spine_03", "upper_lumbar"),
        ),
        entry("spine_03", "upper_lumbar", toward("spine_04", "thoracic")),
        entry("spine_04", "thoracic", toward("neck_01", "cervical")),
        entry("spine_05", "thoracic", toward("neck_01", "cervical")),
        entry("neck_01", "cervical", toward("head", "cranium")),
        entry("neck_02", "cervical", toward("head", "cranium")),
        entry("head", "cranium", Align::Rest),
    ];
    for side in ["l", "r"] {
        let j = side;
        map.extend([
            entry(
                &format!("upperarm_{side}"),
                &format!("{j}humerus"),
                toward(&format!("lowerarm_{side}"), &format!("{j}radius")),
            ),
            entry(
                &format!("lowerarm_{side}"),
                &format!("{j}radius"),
                toward(&format!("hand_{side}"), &format!("{j}hand")),
            ),
            entry(
                &format!("hand_{side}"),
                &format!("{j}hand"),
                toward(&format!("middle_01_{side}"), &format!("{j}_d2_j1")),
            ),
            entry(
                &format!("thigh_{side}"),
                &format!("{j}femurYZ"),
                toward(&format!("calf_{side}"), &format!("{j}tibia")),
            ),
            entry(
                &format!("calf_{side}"),
                &format!("{j}tibia"),
                toward(&format!("foot_{side}"), &format!("{j}talus")),
            ),
            entry(&format!("foot_{side}"), &format!("{j}talus"), Align::Rest),
        ]);
        for (finger, group) in [
            ("thumb", "d1"),
            ("index", "d2"),
            ("middle", "d2"),
            ("ring", "d4"),
            ("pinky", "d4"),
        ] {
            let first = format!("{finger}_01_{side}");
            map.push(entry(
                &first,
                &format!("{j}_{group}_j1"),
                toward(&format!("{finger}_02_{side}"), &format!("{j}_{group}_j2")),
            ));
            for joint in ["02", "03"] {
                map.push(entry(
                    &format!("{finger}_{joint}_{side}"),
                    &format!("{j}_{group}_j2"),
                    Align::As(first.clone()),
                ));
            }
        }
    }
    map
}

/// The parsed manifest.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Manifest {
    /// The glTF file, relative to the model folder.
    pub(crate) model: Option<String>,
    /// The stock model whose GLM and skeleton drive the player.
    pub(crate) driver: String,
    pub(crate) preset: String,
    /// glTF metres to game units; `None` fits the pelvis to the driver's.
    pub(crate) scale: Option<f32>,
    pub(crate) map: Vec<MapEntry>,
}

impl Manifest {
    /// Read `text`: one `key value...` per line, `//` comments.
    pub(crate) fn parse(text: &str) -> Result<Self, String> {
        let mut manifest = Self {
            model: None,
            driver: DEFAULT_DRIVER.to_owned(),
            preset: "mannequin".to_owned(),
            scale: None,
            map: Vec::new(),
        };
        for (number, line) in text.lines().enumerate() {
            let line = line.split("//").next().unwrap_or("").trim();
            let words: Vec<&str> = line.split_whitespace().collect();
            let bad = || format!("{MANIFEST} line {}: {line:?}", number + 1);
            match words.as_slice() {
                [] => {}
                ["model", file] => manifest.model = Some((*file).to_owned()),
                ["driver", directory] => {
                    manifest.driver = directory.trim_end_matches('/').to_owned()
                }
                ["preset", name] => manifest.preset = (*name).to_owned(),
                ["scale", "auto"] => manifest.scale = None,
                ["scale", value] => {
                    manifest.scale = Some(
                        value
                            .parse()
                            .ok()
                            .filter(|s: &f32| *s > 0.0)
                            .ok_or_else(bad)?,
                    )
                }
                ["map", joint, bone] => manifest.map.push(entry(joint, bone, Align::Rest)),
                ["map", joint, bone, "as", other] => {
                    manifest
                        .map
                        .push(entry(joint, bone, Align::As((*other).to_owned())))
                }
                ["map", joint, bone, joint_end, bone_end] => {
                    manifest
                        .map
                        .push(entry(joint, bone, toward(joint_end, bone_end)))
                }
                _ => return Err(bad()),
            }
        }
        Ok(manifest)
    }

    /// The preset's entries with this manifest's `map` lines replacing or adding.
    fn bone_map(&self) -> Result<Vec<MapEntry>, String> {
        let preset = PRESETS
            .iter()
            .find(|(name, _)| name.eq_ignore_ascii_case(&self.preset))
            .ok_or_else(|| format!("{MANIFEST}: unknown preset {:?}", self.preset))?;
        let mut map = (preset.1)();
        for line in &self.map {
            match map.iter_mut().find(|e| e.joint == line.joint) {
                Some(existing) => *existing = line.clone(),
                None => map.push(line.clone()),
            }
        }
        Ok(map)
    }
}

/// The manifest of the model in `directory`, when it is a rig model.
pub(crate) fn manifest(
    vfs: &VirtualFileSystem,
    directory: &str,
) -> Result<Option<Manifest>, Box<dyn Error>> {
    let Some(asset) = vfs.read(&format!("{directory}/{MANIFEST}"))? else {
        return Ok(None);
    };
    Ok(Some(Manifest::parse(&String::from_utf8_lossy(
        &asset.bytes,
    ))?))
}

/// One rig joint, in the driver's model space and units.
#[derive(Clone, Debug)]
pub(crate) struct Joint {
    parent: Option<usize>,
    bind: Affine3A,
    inverse_bind: Affine3A,
    /// The bind pose relative to the parent's.
    local: Affine3A,
}

/// A joint that follows a driver bone.
#[derive(Clone, Copy, Debug)]
struct Driven {
    bone: usize,
    /// The alignment onto the JKA base pose, times the bind rotation.
    rotation: Mat3A,
    /// The pelvis also follows the bone's translation from its base pose.
    translate: Option<Vec3A>,
}

/// One drawn surface, its vertices in the driver's model space.
#[derive(Clone, Debug)]
pub(crate) struct Surface {
    pub(crate) shader: String,
    pub(crate) vertices: Vec<Vertex>,
    pub(crate) indices: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Vertex {
    pub(crate) position: [f32; 3],
    pub(crate) normal: [f32; 3],
    pub(crate) texture_coordinates: [f32; 2],
    pub(crate) joints: [u16; 4],
    pub(crate) weights: [f32; 4],
}

impl Vertex {
    /// Influences with weight, counted from the first (they are sorted strongest first).
    pub(crate) fn influences(&self) -> usize {
        self.weights.iter().take_while(|w| **w > 0.0).count().max(1)
    }
}

/// A loaded rig model: joints, bone map and surfaces, shared by every actor wearing it.
#[derive(Debug)]
pub(crate) struct RigModel {
    pub(crate) joints: Vec<Joint>,
    /// Joint indices, parents before children.
    order: Vec<usize>,
    driven: Vec<Option<Driven>>,
    pub(crate) surfaces: Vec<Surface>,
}

/// glTF (+Y up, +Z front) to the GLA's model space (+Z up, -Y front): the axes a
/// Blender export writes, read back.
fn to_game_axes() -> Mat3A {
    Mat3A::from_cols(
        Vec3A::new(1.0, 0.0, 0.0),
        Vec3A::new(0.0, 0.0, 1.0),
        Vec3A::new(0.0, -1.0, 0.0),
    )
}

fn affine(rows: &[[f32; 4]; 3]) -> Affine3A {
    Affine3A::from_cols_array(&[
        rows[0][0], rows[1][0], rows[2][0], rows[0][1], rows[1][1], rows[2][1], rows[0][2],
        rows[1][2], rows[2][2], rows[0][3], rows[1][3], rows[2][3],
    ])
}

fn rows(matrix: &Affine3A) -> [[f32; 4]; 3] {
    let m = matrix.matrix3;
    let t = matrix.translation;
    [
        [m.x_axis.x, m.y_axis.x, m.z_axis.x, t.x],
        [m.x_axis.y, m.y_axis.y, m.z_axis.y, t.y],
        [m.x_axis.z, m.y_axis.z, m.z_axis.z, t.z],
    ]
}

/// An orthonormal frame whose X runs along `direction`, Z perpendicular to it and
/// the front (-Y), so two such frames differ only by how the bone is aimed.
fn frame(direction: Vec3A) -> Option<Mat3A> {
    let x = direction.try_normalize()?;
    let z = x.cross(Vec3A::NEG_Y).try_normalize()?;
    Some(Mat3A::from_cols(x, z.cross(x), z))
}

impl RigModel {
    /// Load the glTF named by `manifest` from `directory` and fit it to `driver`.
    pub(crate) fn load(
        vfs: &VirtualFileSystem,
        directory: &str,
        manifest: &Manifest,
        driver: &Gla,
    ) -> Result<Self, Box<dyn Error>> {
        let read = |path: &str| vfs.read(path).ok().flatten().map(|asset| asset.bytes);
        let file = match &manifest.model {
            Some(file) => file.clone(),
            None if vfs.contains(&format!("{directory}/model.gltf"))? => "model.gltf".to_owned(),
            None => "model.glb".to_owned(),
        };
        let path = format!("{directory}/{file}");
        let base = path
            .rsplit_once('/')
            .map_or("", |(base, _)| base)
            .to_owned();
        let bytes = read(&path).ok_or_else(|| format!("rig model {path:?} was not found"))?;
        let model = parse(&bytes, |uri| read(&join(&base, uri)))?;
        Self::fit(model, &base, manifest, driver)
    }

    fn fit(
        model: GltfModel,
        base: &str,
        manifest: &Manifest,
        driver: &Gla,
    ) -> Result<Self, Box<dyn Error>> {
        let map = manifest.bone_map()?;
        let bone_index = |name: &str| driver.bones.iter().position(|b| b.name == name);
        let joint_index = |name: &str| model.joints.iter().position(|j| j.name == name);
        let bone_base = |bone: usize| affine(&driver.bones[bone].base_pose);
        let axes = to_game_axes();
        // Scale: the pelvis joint's bind height onto the driver's pelvis base height.
        let pelvis = map.iter().find(|e| e.bone == "pelvis");
        let scale = match manifest.scale {
            Some(scale) => scale,
            None => {
                let joint = pelvis
                    .and_then(|e| joint_index(&e.joint))
                    .ok_or("scale auto needs a joint mapped to the pelvis")?;
                let bone = bone_index("pelvis").ok_or("the driver skeleton has no pelvis")?;
                let height = model.joints[joint].bind[1][3];
                if height <= 0.0 {
                    return Err("the pelvis joint is not above the model's origin".into());
                }
                bone_base(bone).translation.z / height
            }
        };
        let convert = |m: &Affine3A| Affine3A {
            matrix3: axes * m.matrix3 * axes.transpose(),
            translation: axes * m.translation * scale,
        };
        let mut joints = Vec::with_capacity(model.joints.len());
        for joint in &model.joints {
            let bind = convert(&affine(&joint.bind));
            joints.push(Joint {
                parent: joint.parent,
                bind,
                inverse_bind: bind.inverse(),
                local: bind,
            });
        }
        for index in 0..joints.len() {
            if let Some(parent) = joints[index].parent {
                joints[index].local = joints[parent].inverse_bind * joints[index].bind;
            }
        }
        let order = topological(&joints)?;
        // Alignments first (an `As` entry reads another's), then the driven joints.
        let mut aligned: Vec<Option<Mat3A>> = vec![None; joints.len()];
        for entry in &map {
            let (Some(joint), Some(bone)) = (joint_index(&entry.joint), bone_index(&entry.bone))
            else {
                continue;
            };
            aligned[joint] = Some(match &entry.align {
                Align::Rest | Align::As(_) => Mat3A::IDENTITY,
                Align::Toward {
                    joint_end,
                    bone_end,
                } => {
                    let (Some(joint_end), Some(bone_end)) =
                        (joint_index(joint_end), bone_index(bone_end))
                    else {
                        continue;
                    };
                    let jka = bone_base(bone_end).translation - bone_base(bone).translation;
                    let rig = joints[joint_end].bind.translation - joints[joint].bind.translation;
                    match (frame(jka), frame(rig)) {
                        (Some(jka), Some(rig)) => jka * rig.transpose(),
                        _ => Mat3A::IDENTITY,
                    }
                }
            });
        }
        for entry in &map {
            if let Align::As(other) = &entry.align
                && let (Some(joint), Some(other)) = (joint_index(&entry.joint), joint_index(other))
            {
                aligned[joint] = aligned[other].or(Some(Mat3A::IDENTITY));
            }
        }
        let mut driven = vec![None; joints.len()];
        let mut mapped = 0;
        for entry in &map {
            let (Some(joint), Some(bone)) = (joint_index(&entry.joint), bone_index(&entry.bone))
            else {
                continue;
            };
            mapped += 1;
            let align = aligned[joint].unwrap_or(Mat3A::IDENTITY);
            driven[joint] = Some(Driven {
                bone,
                rotation: align * joints[joint].bind.matrix3,
                translate: (entry.bone == "pelvis").then(|| bone_base(bone).translation),
            });
        }
        if mapped == 0 {
            return Err(format!(
                "no joint of the rig matches the {:?} bone map",
                manifest.preset
            )
            .into());
        }
        let normals = Mat3::from(axes);
        let surfaces = model
            .primitives
            .into_iter()
            .map(|primitive| {
                let vertices = primitive
                    .positions
                    .iter()
                    .enumerate()
                    .map(|(index, position)| {
                        let position = axes * Vec3A::from_array(*position) * scale;
                        let normal = normals
                            * Vec3::from_array(
                                primitive
                                    .normals
                                    .get(index)
                                    .copied()
                                    .unwrap_or([0.0, 1.0, 0.0]),
                            );
                        Vertex {
                            position: position.to_array(),
                            normal: normal.normalize_or_zero().to_array(),
                            texture_coordinates: primitive
                                .texture_coordinates
                                .get(index)
                                .copied()
                                .unwrap_or_default(),
                            joints: primitive.joints.get(index).copied().unwrap_or_default(),
                            weights: primitive
                                .weights
                                .get(index)
                                .copied()
                                .unwrap_or([1.0, 0.0, 0.0, 0.0]),
                        }
                    })
                    .collect::<Vec<_>>();
                if vertices
                    .iter()
                    .flat_map(|v| v.joints)
                    .any(|j| usize::from(j) >= joints.len())
                {
                    return Err("a vertex names a joint the skin does not have".into());
                }
                let shader = primitive
                    .base_color_image
                    .as_deref()
                    .map_or_else(|| UNTEXTURED_SHADER.to_owned(), |uri| join(base, uri));
                Ok(Surface {
                    shader,
                    vertices,
                    indices: primitive.indices,
                })
            })
            .collect::<Result<Vec<_>, Box<dyn Error>>>()?;
        crate::log::progress(format_args!(
            "rig model {base}: {} joints ({mapped} driven by {:?}), {} surfaces, scale {scale:.2}",
            joints.len(),
            manifest.preset,
            surfaces.len(),
        ));
        Ok(Self {
            joints,
            order,
            driven,
            surfaces,
        })
    }

    /// The palette size: one matrix per joint.
    pub(crate) fn joint_count(&self) -> usize {
        self.joints.len()
    }

    /// Pose every joint from the driver's evaluated `bones` (one model-space matrix
    /// per GLA bone, as the GLM is skinned with) into `scratch.palette`.
    pub(crate) fn pose(&self, bones: &[[[f32; 4]; 3]], scratch: &mut Scratch) {
        let Scratch { pose, palette } = scratch;
        self.pose_into(bones, pose, palette);
    }

    fn pose_into(
        &self,
        bones: &[[[f32; 4]; 3]],
        scratch: &mut Vec<Affine3A>,
        palette: &mut Vec<[[f32; 4]; 3]>,
    ) {
        scratch.clear();
        scratch.resize(self.joints.len(), Affine3A::IDENTITY);
        palette.clear();
        palette.resize(self.joints.len(), [[0.0; 4]; 3]);
        for &index in &self.order {
            let joint = &self.joints[index];
            let mut pose = match joint.parent {
                Some(parent) => scratch[parent] * joint.local,
                None => joint.bind,
            };
            if let Some(driven) = self.driven[index]
                && let Some(bone) = bones.get(driven.bone)
            {
                let bone = affine(bone);
                pose.matrix3 = bone.matrix3 * driven.rotation;
                if let Some(base) = driven.translate {
                    pose.translation = joint.bind.translation + bone.transform_point3a(base) - base;
                }
            }
            scratch[index] = pose;
            palette[index] = rows(&(pose * joint.inverse_bind));
        }
    }
}

/// One actor's rig pose storage, kept between frames so posing allocates nothing.
#[derive(Clone, Debug, Default)]
pub(crate) struct Scratch {
    pose: Vec<Affine3A>,
    /// This frame's skinning matrices, one per joint.
    pub(crate) palette: Vec<[[f32; 4]; 3]>,
}

/// `vertex` skinned with `palette` (the CPU path): position and unit normal.
pub(crate) fn skin(vertex: &Vertex, palette: &[[[f32; 4]; 3]]) -> ([f32; 3], [f32; 3]) {
    let mut position = [0.0f32; 3];
    let mut normal = [0.0f32; 3];
    for slot in 0..vertex.influences() {
        let Some(m) = palette.get(usize::from(vertex.joints[slot])) else {
            continue;
        };
        let weight = vertex.weights[slot];
        let (p, n) = (vertex.position, vertex.normal);
        for row in 0..3 {
            position[row] +=
                weight * (m[row][0] * p[0] + m[row][1] * p[1] + m[row][2] * p[2] + m[row][3]);
            normal[row] += weight * (m[row][0] * n[0] + m[row][1] * n[1] + m[row][2] * n[2]);
        }
    }
    let length = (normal[0] * normal[0] + normal[1] * normal[1] + normal[2] * normal[2]).sqrt();
    if length > f32::EPSILON {
        normal = normal.map(|value| value / length);
    }
    (position, normal)
}

/// `uri` relative to the folder `base`, with `./` and backslashes normalised.
fn join(base: &str, uri: &str) -> String {
    let uri = uri.replace('\\', "/");
    let uri = uri.trim_start_matches("./");
    if base.is_empty() {
        uri.to_owned()
    } else {
        format!("{base}/{uri}")
    }
}

/// Joint indices with every parent before its children.
fn topological(joints: &[Joint]) -> Result<Vec<usize>, Box<dyn Error>> {
    let mut order = Vec::with_capacity(joints.len());
    let mut state = vec![0u8; joints.len()];
    fn visit(
        index: usize,
        joints: &[Joint],
        state: &mut [u8],
        order: &mut Vec<usize>,
    ) -> Result<(), Box<dyn Error>> {
        match state[index] {
            2 => return Ok(()),
            1 => return Err("the rig's joint hierarchy has a cycle".into()),
            _ => {}
        }
        state[index] = 1;
        if let Some(parent) = joints[index].parent {
            visit(parent, joints, state, order)?;
        }
        state[index] = 2;
        order.push(index);
        Ok(())
    }
    for index in 0..joints.len() {
        visit(index, joints, &mut state, &mut order)?;
    }
    Ok(order)
}

#[cfg(test)]
#[path = "rig_model_tests.rs"]
mod tests;
