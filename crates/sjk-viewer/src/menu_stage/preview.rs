//! The classic profile's live model preview, where retail drew its
//! `ITEM_TYPE_MODEL` character (`player2.menu`) and JoF EJK its cosmetics
//! preview: the stage actor, with what it wears, drawn into a target of its
//! own with its own camera, then encoded into an 8-bit texture the UI draws
//! as a quad ([`crate::ui_renderer::PREVIEW_TEXTURE`]).
//!
//! The colour target has the scene's format and a `Depth32Float` depth, so
//! the world material pipelines draw into it unchanged; `preview.wgsl` makes
//! the display values and takes coverage from depth. The camera frames the
//! whole body from in front and turns around it at retail's
//! `model_rotation 50` (a degree every 50 ms). The actor is lit by the map
//! where it stands, as the stage model is.

use super::*;
use crate::camera_uniform::CameraUniform;

/// Turn rate around the model (`model_rotation 50`).
const TURN_DEGREES_PER_SECOND: f32 = 20.0;
/// Vertical field of view.
const FIELD_OF_VIEW_DEGREES: f32 = 30.0;
/// Half the height and half the width framed around the model's middle.
const HALF_HEIGHT: f32 = 40.0;
const HALF_WIDTH: f32 = 26.0;
/// Height of the framed middle above the actor origin (a player's box runs
/// from 24 below it to 40 above).
const MIDDLE_ABOVE_ORIGIN: f32 = 8.0;
/// Longest side of the target, and the step its sides are rounded up to.
const MAX_SIDE: u32 = 1_024;
const SIDE_STEP: u32 = 16;
/// Camera flags (`CameraUniform::_padding`): a secondary view without probe
/// reflections.
const VIEW_FLAGS: f32 = 17.0;

/// The encode pipeline, made for one scene format.
struct Encode {
    format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
}

/// One target size's textures and bindings.
struct Target {
    size: [u32; 2],
    color: wgpu::TextureView,
    depth: wgpu::TextureView,
    display: wgpu::TextureView,
    camera: wgpu::Buffer,
    camera_bind: wgpu::BindGroup,
    encode_bind: wgpu::BindGroup,
}

/// Preview state on [`MenuStage`].
#[derive(Default)]
pub(crate) struct Preview {
    encode: Option<Encode>,
    target: Option<Target>,
    /// The preview's size on screen in pixels, while the UI shows one.
    pub(super) wanted: Option<[u32; 2]>,
    started: Option<Instant>,
    /// The target holds a drawn frame the UI can show.
    pub(super) ready: bool,
}

/// The target size for `wanted` pixels on screen: no side over
/// [`MAX_SIDE`], the aspect kept, sides rounded up to [`SIDE_STEP`].
pub(super) fn target_size([width, height]: [u32; 2]) -> [u32; 2] {
    let longest = width.max(height).max(1);
    let scale = (MAX_SIDE as f32 / longest as f32).min(1.0);
    [width, height].map(|side| {
        let side = (side as f32 * scale).ceil().max(SIDE_STEP as f32) as u32;
        side.div_ceil(SIDE_STEP) * SIDE_STEP
    })
}

/// The camera for an actor at `origin` facing `facing`, `seconds` into the
/// turn, for a target of `aspect` (width over height).
pub(super) fn camera(origin: Vec3, facing: Vec3, aspect: f32, seconds: f32) -> (Mat4, Vec3, Vec3) {
    let middle = origin + Vec3::Z * MIDDLE_ABOVE_ORIGIN;
    let half = (FIELD_OF_VIEW_DEGREES * 0.5).to_radians().tan();
    let distance = (HALF_HEIGHT / half).max(HALF_WIDTH / (half * aspect.max(0.1)));
    let turn = Quat::from_rotation_z((seconds * TURN_DEGREES_PER_SECOND).to_radians());
    let facing = Vec3::new(facing.x, facing.y, 0.0).normalize_or(Vec3::X);
    let eye = middle + turn * facing * distance + Vec3::Z * 4.0;
    let view = look_at_mat4(eye, middle, Vec3::Z);
    let projection = perspective(
        FIELD_OF_VIEW_DEGREES.to_radians(),
        aspect,
        2.0,
        distance * 4.0,
    );
    (projection * view, eye, (middle - eye).normalize_or(Vec3::X))
}

impl Encode {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("SJK model preview encode"),
            source: wgpu::ShaderSource::Wgsl(include_str!("preview.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK model preview encode layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    },
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    count: None,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Depth,
                    },
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let encoding = if matches!(
            format,
            wgpu::TextureFormat::Rgba16Float | wgpu::TextureFormat::Rg11b10Ufloat
        ) {
            2.0
        } else if format.is_srgb() {
            1.0
        } else {
            0.0
        };
        let constants = [("ENCODING", encoding)];
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SJK model preview encode"),
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
                compilation_options: wgpu::PipelineCompilationOptions {
                    constants: &constants,
                    ..Default::default()
                },
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
        Self {
            format,
            pipeline,
            layout,
        }
    }
}

impl Target {
    fn new(
        device: &wgpu::Device,
        encode: &Encode,
        camera_layout: &wgpu::BindGroupLayout,
        size: [u32; 2],
    ) -> Self {
        let texture = |label, format, usage| {
            device
                .create_texture(&wgpu::TextureDescriptor {
                    label: Some(label),
                    size: wgpu::Extent3d {
                        width: size[0],
                        height: size[1],
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage,
                    view_formats: &[],
                })
                .create_view(&Default::default())
        };
        let attachment =
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
        let color = texture("SJK model preview colour", encode.format, attachment);
        let depth = texture("SJK model preview depth", DepthTarget::FORMAT, attachment);
        let display = texture(
            "SJK model preview display",
            crate::ui_target::TEXTURE_FORMAT,
            attachment,
        );
        let camera = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK model preview camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK model preview camera"),
            layout: camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera.as_entire_binding(),
            }],
        });
        let encode_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK model preview encode"),
            layout: &encode.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&color),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&depth),
                },
            ],
        });
        Self {
            size,
            color,
            depth,
            display,
            camera,
            camera_bind,
            encode_bind,
        }
    }
}

impl GpuState {
    /// Draw the preview for this frame when the UI shows one: the stage
    /// actor into the preview target with the preview camera, then the
    /// display texture the UI samples.
    pub(crate) fn encode_stage_preview(&mut self, encoder: &mut wgpu::CommandEncoder) {
        let wanted = self
            .menu_stage
            .preview
            .wanted
            .filter(|_| self.menu_stage.preview_only && self.menu_stage.actor.is_some());
        let Some(wanted) = wanted else {
            self.menu_stage.preview.ready = false;
            self.menu_stage.preview.started = None;
            return;
        };
        let format = self.context.scene_format();
        let preview = &mut self.menu_stage.preview;
        if preview
            .encode
            .as_ref()
            .is_none_or(|encode| encode.format != format)
        {
            preview.encode = Some(Encode::new(&self.device, format));
            preview.target = None;
        }
        let size = target_size(wanted);
        if preview
            .target
            .as_ref()
            .is_none_or(|target| target.size != size)
        {
            let Some(encode) = preview.encode.as_ref() else {
                return;
            };
            let target = Target::new(&self.device, encode, &self.camera_layout, size);
            self.ui_shapes.set_preview(&self.device, &target.display);
            preview.target = Some(target);
            preview.ready = false;
        }
        let started = *preview.started.get_or_insert_with(Instant::now);
        let (Some(actor), Some(target), Some(encode)) = (
            self.menu_stage.actor.as_ref(),
            self.menu_stage.preview.target.as_ref(),
            self.menu_stage.preview.encode.as_ref(),
        ) else {
            return;
        };
        // The model faces its yaw; the actor's rotation includes the Ghoul2
        // facing turn (`weapon_view::actor_world_rotation`).
        let facing = actor.rotation * Quat::from_rotation_z(-std::f32::consts::FRAC_PI_2) * Vec3::X;
        let aspect = target.size[0] as f32 / target.size[1] as f32;
        let (view_projection, eye, forward) = camera(
            actor.origin,
            facing,
            aspect,
            started.elapsed().as_secs_f32(),
        );
        let uniform = CameraUniform {
            view_projection: view_projection.to_cols_array_2d(),
            camera_position: eye.to_array(),
            shader_time: crate::menu::art::motion::seconds() as f32,
            view_forward: forward.to_array(),
            _padding: VIEW_FLAGS,
        };
        self.queue
            .write_buffer(&target.camera, 0, bytemuck::bytes_of(&uniform));
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK model preview"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &target.color,
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
            for blended in [false, true] {
                self.menu_stage.draw_parts(
                    &mut pass,
                    &self.world_materials,
                    &target.camera_bind,
                    blended,
                );
            }
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("SJK model preview encode"),
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
            pass.set_pipeline(&encode.pipeline);
            pass.set_bind_group(0, &target.encode_bind, &[]);
            pass.draw(0..3, 0..1);
        }
        self.menu_stage.preview.ready = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_encode_program_validates() {
        crate::wgsl_source::validate(include_str!("preview.wgsl"));
    }

    #[test]
    fn targets_keep_the_aspect_within_the_largest_side() {
        assert_eq!(target_size([990, 990]), [992, 992]);
        assert_eq!(target_size([2000, 1000]), [1024, 512]);
        assert_eq!(target_size([10, 400]), [16, 400]);
        assert_eq!(target_size([0, 0]), [16, 16]);
    }

    #[test]
    fn the_camera_frames_the_model_from_in_front() {
        let origin = Vec3::new(100.0, 50.0, 24.0);
        let (view_projection, eye, forward) = camera(origin, Vec3::X, 1.0, 0.0);
        // In front (on the facing side) and looking back at it.
        assert!(eye.x > origin.x + 100.0);
        assert!(forward.x < -0.9);
        // Head and feet both land inside the view.
        for z in [origin.z - 24.0, origin.z + 40.0] {
            let clip = view_projection * Vec3::new(origin.x, origin.y, z).extend(1.0);
            let ndc = clip.truncate() / clip.w;
            assert!(ndc.y.abs() < 1.0 && ndc.x.abs() < 1.0, "{z}: {ndc}");
        }
        // A quarter turn later the camera has moved round to the side.
        let (_, eye, _) = camera(origin, Vec3::X, 1.0, 90.0 / TURN_DEGREES_PER_SECOND);
        assert!((eye.y - origin.y).abs() > 100.0);
    }
}
