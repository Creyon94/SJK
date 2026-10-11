//! The invisible faces of a map, rebuilt from its brushes' planes: q3map2 keeps
//! every brush for collision but makes no drawn surface for a tool texture.

use super::kinds::Kind;
use bytemuck::{Pod, Zeroable};
use sjk_bsp::Bsp;
use std::ops::Range;

/// Vertex shared by faces and edges; layout of `brush_view.wgsl`'s `VertexIn`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub(super) struct Vertex {
    pub(super) position: [f32; 3],
    /// The face's plane normal, pointing out of its brush.
    pub(super) normal: [f32; 3],
    pub(super) kind: u32,
}

/// A face smaller than this (square units) is a bevel or sliver; it is skipped.
const MIN_AREA: f32 = 0.5;

/// Every invisible face as triangles and outline segments, grouped by kind.
#[derive(Default)]
pub(super) struct Mesh {
    pub(super) faces: Vec<Vertex>,
    pub(super) face_ranges: Vec<Range<u32>>,
    pub(super) edges: Vec<Vertex>,
    pub(super) edge_ranges: Vec<Range<u32>>,
}

impl Mesh {
    /// Build from the world's brushes and those of `trigger_*` entities. Other
    /// entities' brushes (doors, lifts, glass) are left out: they may move.
    pub(super) fn build(bsp: &Bsp) -> Self {
        let mut faces: Vec<Vec<Vertex>> = vec![Vec::new(); Kind::ALL.len()];
        let mut edges: Vec<Vec<Vertex>> = vec![Vec::new(); Kind::ALL.len()];
        let models = bsp.render().models();
        let triggers = trigger_models(bsp, models.len());
        for (model_index, model) in models.iter().enumerate() {
            let trigger = if model_index == 0 {
                None
            } else if let Some(kind) = triggers[model_index] {
                Some(kind)
            } else {
                continue;
            };
            for brush_index in model.brushes.clone() {
                let Some(brush) = bsp.brushes().get(brush_index) else {
                    continue;
                };
                for side_index in brush.sides.clone() {
                    let Some(side) = bsp.brush_sides().get(side_index) else {
                        continue;
                    };
                    let Some(kind) = trigger.or_else(|| {
                        let shader = bsp.shaders().get(side.shader)?;
                        Kind::of_face(brush.content_flags, shader.surface_flags, shader.name())
                    }) else {
                        continue;
                    };
                    let points = bsp.brush_side_polygon(brush_index, side_index);
                    if area(&points) < MIN_AREA {
                        continue;
                    }
                    let normal = bsp.planes()[side.plane].normal;
                    let vertex = |position| Vertex {
                        position,
                        normal,
                        kind: kind.index() as u32,
                    };
                    let bucket = &mut faces[kind.index()];
                    for i in 2..points.len() {
                        bucket.extend([points[0], points[i - 1], points[i]].map(vertex));
                    }
                    let bucket = &mut edges[kind.index()];
                    for (i, &point) in points.iter().enumerate() {
                        bucket.extend([point, points[(i + 1) % points.len()]].map(vertex));
                    }
                }
            }
        }
        let (faces, face_ranges) = concatenate(faces);
        let (edges, edge_ranges) = concatenate(edges);
        Self {
            faces,
            face_ranges,
            edges,
            edge_ranges,
        }
    }
}

/// The trigger kind of each inline model a `trigger_*` entity uses.
fn trigger_models(bsp: &Bsp, count: usize) -> Vec<Option<Kind>> {
    let mut triggers = vec![None; count];
    let entities = sjk_entity::parse_entity_lump(bsp.entities()).unwrap_or_default();
    for entity in &entities {
        let Some(classname) = entity
            .classname()
            .filter(|name| name.len() > 8 && name[..8].eq_ignore_ascii_case("trigger_"))
        else {
            continue;
        };
        if let Some(slot) = entity
            .get("model")
            .and_then(|model| model.strip_prefix('*'))
            .and_then(|index| index.parse::<usize>().ok())
            .filter(|&index| index > 0)
            .and_then(|index| triggers.get_mut(index))
        {
            *slot = Some(Kind::of_trigger(classname));
        }
    }
    triggers
}

fn concatenate(buckets: Vec<Vec<Vertex>>) -> (Vec<Vertex>, Vec<Range<u32>>) {
    let mut all = Vec::with_capacity(buckets.iter().map(Vec::len).sum());
    let mut ranges = Vec::with_capacity(buckets.len());
    for bucket in buckets {
        let start = all.len() as u32;
        all.extend(bucket);
        ranges.push(start..all.len() as u32);
    }
    (all, ranges)
}

fn area(points: &[[f32; 3]]) -> f32 {
    let mut sum = [0.0f32; 3];
    for i in 2..points.len() {
        let a = sub(points[i - 1], points[0]);
        let b = sub(points[i], points[0]);
        let c = [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ];
        for axis in 0..3 {
            sum[axis] += c[axis];
        }
    }
    0.5 * (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn area_of_a_square() {
        let square = [
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [2.0, 2.0, 0.0],
            [0.0, 2.0, 0.0],
        ];
        assert!((area(&square) - 4.0).abs() < 1e-5);
        assert_eq!(area(&square[..2]), 0.0);
    }

    #[test]
    fn buckets_become_ranges_in_kind_order() {
        let vertex = |kind| Vertex {
            position: [0.0; 3],
            normal: [0.0, 0.0, 1.0],
            kind,
        };
        let (all, ranges) = concatenate(vec![vec![vertex(0); 3], vec![], vec![vertex(2); 6]]);
        assert_eq!(all.len(), 9);
        assert_eq!(ranges, [0..3, 3..3, 3..9]);
    }
}
