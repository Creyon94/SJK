//! Cached CG_ForcePushBodyBlur joint indices; no second pose evaluation.
use glam::Vec3;
use sjk_model::Gla;

const NAMES: [&str; 8] = [
    "cranium",
    "lower_lumbar",
    "rhand",
    "lhand",
    "ltibia",
    "rtibia",
    "lradius",
    "rradius",
];

/// Fixed joint-origin storage populated alongside the existing saber bolts.
pub(crate) struct ForceBones {
    indices: [Option<usize>; 8],
    /// Model-space joints from the latest evaluated skin pose; missing bones stay absent.
    pub(crate) origins: [Option<Vec3>; 8],
    /// Raw lower-lumbar bolt from this pose, shared by carried-flag submission.
    pub(crate) lumbar: Option<crate::bolt::BoltMatrix>,
    /// `ceyebrow`, stock's head bolt when a model has no `*head_top` tag
    /// (`cg_players.c:671-675`).
    eyebrow: Option<usize>,
    /// Model-space head bolt (`ci->bolt_head`) from the latest pose, for the confusion
    /// effect over a mind-tricked player's head.
    pub(crate) head: Option<Vec3>,
}

impl ForceBones {
    /// Resolve names once while loading an actor (cg_players.c:4995-5006).
    pub(crate) fn new(animation: &Gla) -> Self {
        Self {
            indices: NAMES.map(|name| {
                animation
                    .bones
                    .iter()
                    .position(|bone| bone.name.eq_ignore_ascii_case(name))
            }),
            origins: [None; 8],
            lumbar: None,
            eyebrow: animation
                .bones
                .iter()
                .position(|bone| bone.name.eq_ignore_ascii_case("ceyebrow")),
            head: None,
        }
    }

    /// Undo the inverse bind pose, exactly as the existing generic joint query does.
    pub(crate) fn update(&mut self, animation: &Gla, matrices: &[[[f32; 4]; 3]]) {
        // G2_GetBoltMatrixLow, tr_ghoul2.cpp:3132: skin matrix * base pose.
        self.lumbar = self.indices[1].and_then(|index| {
            let matrix = matrices.get(index)?;
            let bind = animation.bones.get(index)?.base_pose;
            Some(std::array::from_fn(|row| {
                std::array::from_fn(|column| {
                    (0..3)
                        .map(|axis| matrix[row][axis] * bind[axis][column])
                        .sum::<f32>()
                        + if column == 3 { matrix[row][3] } else { 0.0 }
                })
            }))
        });
        for (output, index) in self.origins.iter_mut().zip(self.indices) {
            *output = index.and_then(|index| {
                let matrix = matrices.get(index)?;
                let bind = animation.bones.get(index)?.base_pose;
                Some(Vec3::from_array(std::array::from_fn(|row| {
                    matrix[row][3]
                        + (0..3)
                            .map(|axis| matrix[row][axis] * bind[axis][3])
                            .sum::<f32>()
                })))
            });
        }
    }
}

impl ForceBones {
    /// Update [`Self::head`]: the `*head_top` tag, else the `ceyebrow` joint.
    pub(crate) fn update_head(
        &mut self,
        mesh: &sjk_model::Glm,
        animation: &Gla,
        matrices: &[[[f32; 4]; 3]],
    ) {
        self.head = match mesh.surface_bolt_matrix("*head_top", 0, matrices) {
            Ok(Some(bolt)) => Some(Vec3::from_array(crate::bolt::column(&bolt, 3))),
            _ => self.eyebrow.and_then(|index| {
                let matrix = matrices.get(index)?;
                let bind = animation.bones.get(index)?.base_pose;
                Some(Vec3::from_array(std::array::from_fn(|row| {
                    matrix[row][3]
                        + (0..3)
                            .map(|axis| matrix[row][axis] * bind[axis][3])
                            .sum::<f32>()
                })))
            }),
        };
    }
}
