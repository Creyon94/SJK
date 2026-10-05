//! Production WGPU resources for saber blades and motion trails.

use crate::saber::{self, Instance};
use crate::saber_trail::SegmentPool;
use crate::{DepthTarget, texture_layout_entry};
use sjk_shader::ShaderCatalog;
use sjk_vfs::VirtualFileSystem;
use std::error::Error;
use std::ops::Range;

#[path = "saber_glow_integral.rs"]
pub(crate) mod glow_integral;

pub(crate) struct Runtime {
    pipeline: wgpu::RenderPipeline,
    /// The glow capsules alone, for the dynamic glow image: the retail `*_glow` blade
    /// shaders carry `glow`, the `*_line` cores do not (`sabers.shader`).
    glow_pipeline: wgpu::RenderPipeline,
    materials: Vec<wgpu::BindGroup>,
    instance_buffer: wgpu::Buffer,
    ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    trails: crate::saber_trail_gpu::Runtime,
    /// The classic model preview's blades, kept apart from the world's.
    preview_buffer: wgpu::Buffer,
    preview_ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
}

/// Blade instances the model preview can draw: two sabers of eight blades,
/// two instances each.
pub(crate) const PREVIEW_BLADES: usize = 2 * 8 * 2;

impl Runtime {
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        vfs: &VirtualFileSystem,
        shaders: &ShaderCatalog,
        camera_layout: &wgpu::BindGroupLayout,
        format: wgpu::TextureFormat,
    ) -> Result<Self, Box<dyn Error>> {
        let source = include_str!("saber.wgsl");

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("JKR saber shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("JKR saber texture layout"),
            entries: &[
                texture_layout_entry(0),
                texture_layout_entry(1),
                texture_layout_entry(3),
                sampler_entry(2),
                sampler_entry(4),
            ],
        });
        let materials = saber::create_materials(device, queue, vfs, shaders, &texture_layout)?;
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("JKR saber pipeline layout"),
            bind_group_layouts: &[Some(camera_layout), Some(&texture_layout)],
            immediate_size: 0,
        });
        let create = |fragment| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("JKR saber pipeline"),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(Instance::layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(fragment),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::ADDITIVE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DepthTarget::FORMAT,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::LessEqual),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = create("fragment_main");
        let glow_pipeline = create("fragment_glow");
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("JKR saber instances"),
            size: (saber::MAX_BLADE_INSTANCES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let preview_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK model preview saber instances"),
            size: (PREVIEW_BLADES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            pipeline,
            glow_pipeline,
            materials,
            instance_buffer,
            preview_buffer,
            preview_ranges: std::array::from_fn(|_| 0..0),
            ranges: std::array::from_fn(|_| 0..0),
            trails: crate::saber_trail_gpu::Runtime::new(
                device,
                queue,
                vfs,
                shaders,
                camera_layout,
                format,
            )?,
        })
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
        trails: &mut SegmentPool,
        now: i64,
    ) {
        self.ranges = saber::material_ranges(blades);
        if !blades.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(blades));
        }
        self.trails.prepare(queue, trails, now);
    }

    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        self.trails.draw(pass, camera);
        self.draw_blades(pass, camera, &self.pipeline);
    }

    /// Upload the model preview's blades (at most [`PREVIEW_BLADES`]).
    pub(crate) fn prepare_preview(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
    ) {
        let count = blades.len().min(PREVIEW_BLADES);
        let blades = &mut blades[..count];
        self.preview_ranges = saber::material_ranges(blades);
        if !blades.is_empty() {
            queue.write_buffer(&self.preview_buffer, 0, bytemuck::cast_slice(blades));
        }
    }

    /// Draw the model preview's blades with the preview's `camera`.
    pub(crate) fn draw_preview<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        draw_instances(
            pass,
            camera,
            &self.pipeline,
            &self.materials,
            &self.preview_buffer,
            &self.preview_ranges,
        );
    }

    /// Draw into the dynamic glow image: the trails (`saberBlur`/`swordTrail` glow)
    /// unless `blades_only` (`r_DynamicGlow 2`, stock's `RT_SABER_GLOW` test), then the
    /// blades' glow capsules without their cores.
    pub(crate) fn draw_glow<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        blades_only: bool,
    ) {
        if !blades_only {
            self.trails.draw(pass, camera);
        }
        self.draw_blades(pass, camera, &self.glow_pipeline);
    }

    /// Whether the glow image gets any saber this frame.
    pub(crate) fn has_glow(&self, blades_only: bool) -> bool {
        (!blades_only && self.trails.has_draws()) || self.ranges.iter().any(|r| !r.is_empty())
    }

    fn draw_blades<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        pipeline: &'pass wgpu::RenderPipeline,
    ) {
        draw_instances(
            pass,
            camera,
            pipeline,
            &self.materials,
            &self.instance_buffer,
            &self.ranges,
        );
    }

    /// Add this frame's trail quads to the effect layer's screen bounds.
    pub(crate) fn bound_trails(
        &self,
        bounds: &mut crate::frame_target::aa::effects::bounds::Bounds,
    ) {
        for position in self.trails.positions() {
            bounds.sphere(glam::Vec3::from_array(position), 0.0);
        }
    }

    /// Whether this frame has any blade or trail to draw.
    pub(crate) fn has_draws(&self) -> bool {
        self.trails.has_draws() || self.ranges.iter().any(|range| !range.is_empty())
    }
}

/// Draw the blade instances of `buffer`, a run per material.
fn draw_instances<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    camera: &'pass wgpu::BindGroup,
    pipeline: &'pass wgpu::RenderPipeline,
    materials: &'pass [wgpu::BindGroup],
    buffer: &'pass wgpu::Buffer,
    ranges: &[Range<u32>],
) {
    if ranges.iter().all(Range::is_empty) {
        return;
    }
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, camera, &[]);
    pass.set_vertex_buffer(0, buffer.slice(..));
    for (material, instances) in materials.iter().zip(ranges.iter().cloned()) {
        if instances.is_empty() {
            continue;
        }
        pass.set_bind_group(1, material, &[]);
        pass.draw(0..6, instances);
    }
}
