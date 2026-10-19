//! The Collection's blade pictures (Creyon's round 13: its list showed every blade as a
//! drawing that did not look like the blade): each blade skin a list row or card shows is
//! drawn by the game's own blade renderer, live, as it looks in the hand, and the swatch
//! shows that picture.
//!
//! A swatch asks for its skin's picture ([`picture`]) while the UI is built; the frame then
//! draws every skin asked for into a cell of one picture ([`SLOTS`] cells, kept for a skin
//! while it is still asked for), all in one pass with one camera: each blade lies across
//! its cell side on, in the world, at its own place, so their shared pipelines (and a
//! signature's own, once built) draw them as any blade. The blades go into an 8-bit
//! picture in the effect layer's display values, which the UI adds over the swatch's dark
//! glass ([`crate::ui_renderer::BLADE_PICTURES_TEXTURE`]). A swatch whose skin has no cell
//! yet (the first frame, or more swatches than cells) draws the old drawing.

use crate::camera_uniform::CameraUniform;
use glam::Vec3;
use std::sync::Mutex;

/// Cells of the picture: two columns of eight.
const COLUMNS: usize = 2;
const ROWS: usize = 8;
pub(crate) const SLOTS: usize = COLUMNS * ROWS;
/// Blade instances one cell draws at most (glow and core, glints, an echo).
const PER_SLOT: usize = 12;
/// Blade instances the pictures draw at most (`saber_gpu.rs`).
pub(crate) const BLADES: usize = SLOTS * PER_SLOT;
/// A cell in world units: the blade's 40 and room for its tip and effects, and above and
/// below it room for what a signature draws round it.
const CELL: [f32; 2] = [52.0, 26.0];
/// Where the blade starts in its cell and how long it is (a stock blade's 40 units).
pub(crate) const BLADE_START: f32 = 3.0;
pub(crate) const BLADE_LENGTH: f32 = 40.0;
const BLADE_RADIUS: f32 = 3.0;
/// Pixels a world unit.
const PIXELS: f32 = 8.0;
/// The picture's size in pixels.
pub(crate) const SIZE: [u32; 2] = [
    (CELL[0] * PIXELS) as u32 * COLUMNS as u32,
    (CELL[1] * PIXELS) as u32 * ROWS as u32,
];

/// Who asked for a picture this frame and which cell each has.
#[derive(Default)]
struct State {
    /// Skins asked for while this frame's UI was built.
    wanted: Vec<&'static str>,
    /// Each cell's skin and whether its picture has been drawn.
    slots: [Option<(&'static str, bool)>; SLOTS],
}

static STATE: Mutex<State> = Mutex::new(State {
    wanted: Vec::new(),
    slots: [None; SLOTS],
});

fn lock() -> std::sync::MutexGuard<'static, State> {
    STATE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The picture of skin `id` for a swatch: the texture coordinates `[u0, v0, u1, v1]` of
/// its cell, from its left edge ([`BLADE_START`] units before the blade) to its end, the cell's middle `shown` units
/// high (`shown` at most the cell's height), once drawn. Asking keeps (or gets) it a cell.
pub(crate) fn picture(id: &'static str, shown: f32) -> Option<[f32; 4]> {
    let mut state = lock();
    if !state.wanted.contains(&id) {
        state.wanted.push(id);
    }
    let slot = state
        .slots
        .iter()
        .position(|slot| matches!(slot, Some((skin, true)) if *skin == id))?;
    let (column, row) = (slot % COLUMNS, slot / COLUMNS);
    let width = CELL[0] * COLUMNS as f32;
    let height = CELL[1] * ROWS as f32;
    let shown = shown.clamp(1.0, CELL[1]);
    let top = row as f32 * CELL[1] + (CELL[1] - shown) * 0.5;
    Some([
        column as f32 * CELL[0] / width,
        top / height,
        (column as f32 + 1.0) * CELL[0] / width,
        (top + shown) / height,
    ])
}

/// The cells for this frame: the skins asked for since the last frame keep their cells,
/// the others free theirs, and new ones take free cells (not drawn yet). Clears the wishes.
fn assign(state: &mut State) -> [Option<&'static str>; SLOTS] {
    let wanted = std::mem::take(&mut state.wanted);
    for slot in &mut state.slots {
        if slot.is_some_and(|(id, _)| !wanted.contains(&id)) {
            *slot = None;
        }
    }
    for id in wanted {
        if state
            .slots
            .iter()
            .any(|slot| slot.is_some_and(|(skin, _)| skin == id))
        {
            continue;
        }
        if let Some(free) = state.slots.iter_mut().find(|slot| slot.is_none()) {
            *free = Some((id, false));
        }
    }
    state.slots.map(|slot| slot.map(|(id, _)| id))
}

/// Where the blade of cell `slot` starts, in world units (the picture's plane is y = 0,
/// x across, z up).
fn blade_base(slot: usize) -> Vec3 {
    let (column, row) = (slot % COLUMNS, slot / COLUMNS);
    Vec3::new(
        column as f32 * CELL[0] + BLADE_START,
        0.0,
        -(row as f32 + 0.5) * CELL[1],
    )
}

/// The camera seeing every cell side on: x from 0 to the picture's width, z from 0 down.
fn view_projection() -> glam::Mat4 {
    let width = CELL[0] * COLUMNS as f32;
    let height = CELL[1] * ROWS as f32;
    glam::camera::rh::proj::directx::orthographic(0.0, width, -height, 0.0, 1.0, 400.0)
        * glam::camera::rh::view::look_at_mat4(Vec3::new(0.0, -200.0, 0.0), Vec3::ZERO, Vec3::Z)
}

/// The picture's textures and camera on the GPU.
pub(crate) struct Target {
    blades: wgpu::TextureView,
    depth: wgpu::TextureView,
    /// The blades faded toward each cell's edges (`blade_pictures.wgsl`): what the UI shows.
    display: wgpu::TextureView,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    soften: wgpu::RenderPipeline,
    soften_bind: wgpu::BindGroup,
}

impl Target {
    fn new(device: &wgpu::Device, camera_layout: &wgpu::BindGroupLayout) -> Self {
        let texture = |label, format| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: SIZE[0],
                        height: SIZE[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let blades = texture(
            "SJK blade pictures",
            crate::frame_target::aa::effects::FORMAT,
        );
        let depth = texture(
            "SJK blade pictures depth",
            crate::depth_target::DepthTarget::FORMAT,
        );
        let display = texture(
            "SJK blade pictures display",
            crate::ui_target::TEXTURE_FORMAT,
        );
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK blade pictures edges"),
            source: wgpu::ShaderSource::Wgsl(include_str!("blade_pictures.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK blade pictures edges"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                count: None,
                ty: wgpu::BindingType::Texture {
                    multisampled: false,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                },
            }],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let soften = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK blade pictures edges"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: crate::ui_target::TEXTURE_FORMAT,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let soften_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade pictures edges"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&blades),
            }],
        });
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade pictures camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK blade pictures camera"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        Self {
            blades,
            depth,
            display,
            camera,
            camera_bind,
            soften,
            soften_bind,
        }
    }
}

impl crate::GpuState {
    /// Draw this frame's blade pictures: every skin a swatch asked for while the last UI
    /// was built, each in its cell, live.
    pub(crate) fn encode_blade_pictures(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let slots = assign(&mut lock());
        if slots.iter().all(Option::is_none) {
            return;
        }
        if self.blade_pictures.is_none() {
            let target = Target::new(&self.device, &self.camera_layout);
            self.ui_shapes
                .set_blade_pictures(&self.device, &target.display);
            self.blade_pictures = Some(target);
        }
        let seconds = crate::menu::art::motion::seconds();
        let mut blades = Vec::with_capacity(BLADES);
        let mut drawn = [false; SLOTS];
        for (slot, id) in slots.iter().enumerate() {
            let Some(id) = id else {
                continue;
            };
            let Some(skin) = self
                .blade_skins
                .color_with(id, self.looks.own_options_off(id))
            else {
                continue;
            };
            let color = crate::saber_rgb::BladeColor::Skin(skin);
            let blade = crate::saber::Blade {
                base: blade_base(slot).to_array(),
                direction: [1.0, 0.0, 0.0],
                length: BLADE_LENGTH,
                radius: BLADE_RADIUS,
            };
            let seed = slot as u32;
            let start = blades.len();
            blades.extend(
                crate::saber::Instance::pair(blade, color)
                    .map(|instance| instance.with_animation(seconds, seed))
                    .into_iter()
                    .chain(crate::saber::skin_extras(blade, color, seconds, seed))
                    .map(|instance| instance.with_frame([0.0, 0.0, 1.0], BLADE_LENGTH))
                    .take(PER_SLOT),
            );
            drawn[slot] = blades.len() > start;
        }
        let Some(target) = self.blade_pictures.as_ref() else {
            return;
        };
        let eye = Vec3::new(0.0, -200.0, 0.0);
        let uniform = CameraUniform {
            view_projection: view_projection().to_cols_array_2d(),
            camera_position: eye.to_array(),
            shader_time: seconds as f32,
            view_forward: [0.0, 1.0, 0.0],
            // A secondary view, as the model preview's.
            _padding: 17.0,
        };
        self.queue
            .write_buffer(&target.camera, 0, bytemuck::bytes_of(&uniform));
        self.saber_gpu.prepare_pictures(&self.queue, &mut blades);
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade pictures"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.blades,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &target.depth,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            if !blades.is_empty() {
                self.saber_gpu.draw_pictures(&mut pass, &target.camera_bind);
            }
        }
        {
            // Soft edges for every cell, into the picture the UI shows.
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK blade pictures edges"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.display,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&target.soften);
            pass.set_bind_group(0, &target.soften_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        let mut state = lock();
        for (slot, drawn) in state.slots.iter_mut().zip(drawn) {
            if let Some((_, done)) = slot {
                *done = drawn;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skin_keeps_its_cell_while_asked_for_and_frees_it_after() {
        let mut state = State {
            wanted: vec!["saber_a", "saber_b"],
            ..State::default()
        };
        let slots = assign(&mut state);
        assert_eq!(&slots[..2], &[Some("saber_a"), Some("saber_b")]);
        state.slots[0] = Some(("saber_a", true));
        state.wanted = vec!["saber_c", "saber_a"];
        let slots = assign(&mut state);
        // a keeps cell 0 (drawn), b's cell goes to c.
        assert_eq!(&slots[..2], &[Some("saber_a"), Some("saber_c")]);
        assert_eq!(state.slots[0], Some(("saber_a", true)));
        state.wanted.clear();
        assert!(assign(&mut state).iter().all(Option::is_none));
    }

    #[test]
    fn more_skins_than_cells_leave_the_rest_without() {
        let ids: Vec<&'static str> = (0..SLOTS + 3)
            .map(|n| &*Box::leak(format!("saber_{n}").into_boxed_str()))
            .collect();
        let mut state = State {
            wanted: ids.clone(),
            ..State::default()
        };
        let slots = assign(&mut state);
        assert!(slots.iter().all(Option::is_some));
        assert!(!slots.contains(&Some(ids[SLOTS])));
    }

    #[test]
    fn every_cell_lies_inside_the_camera() {
        let matrix = view_projection();
        for slot in 0..SLOTS {
            let base = blade_base(slot);
            for point in [base, base + Vec3::X * BLADE_LENGTH] {
                let clip = matrix * point.extend(1.0);
                let ndc = clip.truncate() / clip.w;
                assert!(ndc.x.abs() < 1.0 && ndc.y.abs() < 1.0, "{slot}: {ndc}");
            }
        }
        // The first cell's blade in the picture's top left.
        let ndc = (matrix * blade_base(0).extend(1.0)).truncate();
        assert!(ndc.x < -0.9 && ndc.y > 0.8, "{ndc}");
    }
}

#[cfg(test)]
mod edge_tests {
    #[test]
    fn the_edge_program_validates() {
        crate::wgsl_source::validate(include_str!("blade_pictures.wgsl"));
    }
}
