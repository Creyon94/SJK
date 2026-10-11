//! The player card's outline: while a player's card is up
//! ([`crate::hud::player_card`]), a ring in the card's accent colour runs around the
//! outside of their body, and only the outside: an arm across the chest or the gap
//! between the legs draws no line.
//!
//! Two steps. After the scene, the body is drawn once more through its usual vertex
//! path into a one-channel mask, against the scene's depth, so the mask holds only the
//! part of the body that is in view (none through walls); the queue keeps that draw
//! apart from every other pass ([`crate::entity_materials::Queue::outline`]). Then the
//! 2D layer, after anti-aliasing and the colour grade so the colour is the card's
//! exactly, draws the ring: every pixel outside the mask within [`RADIUS`] pixels of
//! it, with a soft outer edge, inside a scissor around the player.

use sjk_vfs::{VfsError, VirtualFileSystem};
use wgpu::util::DeviceExt;

/// The mask material: one plain stage, no deforms, so the body's vertices land where
/// the scene put them.
pub(crate) const MASK_SHADER: &str = "sjk/cardOutlineMask";
/// The mask image's format.
pub(crate) const MASK_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R8Unorm;
/// The ring's width in pixels of a 1080-line screen; it scales with the screen height.
pub(crate) const RADIUS: f32 = 4.0;
/// Most mask lookups along one direction (the radius in steps of about a pixel).
const MAX_STEPS: f32 = 16.0;

const SHADER_FILE: &str = "// SJK: the player card outline's mask (card_outline.rs).
sjk/cardOutlineMask
{
\tcull none
\t{
\t\tmap $whiteimage
\t\trgbGen identity
\t}
}
";

/// Mount the shader file, below game data like SJK's other content.
pub(crate) fn mount(vfs: &mut VirtualFileSystem) -> Result<(), VfsError> {
    vfs.mount_memory(
        "SJK card outline",
        [("shaders/sjk_card_outline.shader", SHADER_FILE.as_bytes())],
    )?;
    Ok(())
}

/// The ring's width in output pixels for a screen `height` pixels tall.
pub(crate) fn radius(height: f32) -> f32 {
    (RADIUS * height / 1080.0).clamp(2.0, MAX_STEPS)
}

/// This frame's outline: its colour (display values, alpha the card's fade) and the
/// screen area it may cover, in output pixels (`[x, y, width, height]`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Frame {
    pub(crate) color: [u8; 4],
    pub(crate) area: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniform {
    color: [f32; 4],
    /// Output size in pixels, then the radius in pixels and its step count.
    shape: [f32; 4],
}

/// The mask image, the ring's program and its settings.
pub(crate) struct Gpu {
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    uniform: wgpu::Buffer,
    /// The mask at the scene's depth size, its view and the ring's bind group.
    mask: Option<([u32; 2], wgpu::TextureView, wgpu::BindGroup)>,
    /// What the ring draws this frame, set once the mask is drawn.
    frame: Option<Frame>,
}

impl Gpu {
    /// The ring's program for the 2D layer's `format`; the mask comes with the first
    /// outline.
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("card outline"),
            entries: &[
                crate::render_helpers::texture_layout_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("card outline"),
            source: wgpu::ShaderSource::Wgsl(include_str!("card_outline.wgsl").into()),
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("card outline"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("card outline ring"),
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
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            // The 2D layer's pass carries the scene's depth, never tested here.
            depth_stencil: Some(wgpu::DepthStencilState {
                format: crate::DepthTarget::FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(wgpu::CompareFunction::Always),
                stencil: Default::default(),
                bias: Default::default(),
            }),
            multisample: Default::default(),
            multiview_mask: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("card outline mask"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("card outline"),
            contents: bytemuck::bytes_of(&Uniform {
                color: [0.0; 4],
                shape: [1.0, 1.0, 1.0, 1.0],
            }),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        Self {
            pipeline,
            layout,
            sampler,
            uniform,
            mask: None,
            frame: None,
        }
    }

    /// Prepare this frame's outline: `frame` when a card is up with the outline on, the
    /// scene's depth `size` and the output's. With none, nothing is drawn and the mask
    /// is kept for the next one.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        frame: Option<Frame>,
        size: [u32; 2],
        output: [u32; 2],
    ) {
        self.frame = frame.filter(|frame| frame.color[3] > 0);
        let Some(frame) = self.frame else {
            return;
        };
        if self.mask.as_ref().is_none_or(|(at, _, _)| *at != size) {
            let texture = device.create_texture(&wgpu::TextureDescriptor {
                label: Some("card outline mask"),
                size: wgpu::Extent3d {
                    width: size[0].max(1),
                    height: size[1].max(1),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: MASK_FORMAT,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            });
            let view = texture.create_view(&Default::default());
            let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("card outline"),
                layout: &self.layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.uniform.as_entire_binding(),
                    },
                ],
            });
            self.mask = Some((size, view, group));
        }
        let radius = radius(output[1] as f32);
        let color = frame.color.map(|channel| f32::from(channel) / 255.0);
        queue.write_buffer(
            &self.uniform,
            0,
            bytemuck::bytes_of(&Uniform {
                color,
                shape: [output[0] as f32, output[1] as f32, radius, radius.ceil()],
            }),
        );
    }

    /// The mask to draw the body into this frame, when there is an outline.
    pub(crate) fn mask_target(&self) -> Option<&wgpu::TextureView> {
        self.frame?;
        self.mask.as_ref().map(|(_, view, _)| view)
    }

    /// Draw the ring into the 2D layer's `pass`, `output` pixels large.
    pub(crate) fn draw<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>, output: [u32; 2]) {
        let (Some(frame), Some((_, _, group))) = (self.frame, &self.mask) else {
            return;
        };
        let Some([x, y, width, height]) = scissor(frame.area, output) else {
            return;
        };
        pass.set_scissor_rect(x, y, width, height);
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, group, &[]);
        pass.draw(0..3, 0..1);
        pass.set_scissor_rect(0, 0, output[0], output[1]);
    }
}

/// `area` grown by the ring's width and cut to the `output`, in whole pixels; `None`
/// when nothing of it is on screen.
fn scissor(area: [f32; 4], output: [u32; 2]) -> Option<[u32; 4]> {
    let grow = radius(output[1] as f32) + 2.0;
    let [w, h] = output.map(|value| value as f32);
    let left = (area[0] - grow).clamp(0.0, w);
    let top = (area[1] - grow).clamp(0.0, h);
    let right = (area[0] + area[2] + grow).clamp(0.0, w);
    let bottom = (area[1] + area[3] + grow).clamp(0.0, h);
    if right - left < 1.0 || bottom - top < 1.0 {
        return None;
    }
    let (x, y) = (left.floor() as u32, top.floor() as u32);
    Some([
        x,
        y,
        (right.ceil() as u32).min(output[0]) - x,
        (bottom.ceil() as u32).min(output[1]) - y,
    ])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_mask_shader_is_one_plain_two_sided_stage() {
        let shaders =
            sjk_shader::parse_shader_script(SHADER_FILE.as_bytes(), "card_outline").unwrap();
        let [shader] = shaders.as_slice() else {
            panic!("one shader");
        };
        assert_eq!(shader.name, MASK_SHADER);
        assert_eq!(shader.cull, sjk_shader::ShaderCull::TwoSided);
        assert!(shader.deforms.is_empty());
        assert_eq!(shader.stages.len(), 1);
    }

    #[test]
    fn the_ring_program_validates() {
        crate::wgsl_source::validate(include_str!("card_outline.wgsl"));
    }

    #[test]
    fn the_ring_widens_with_the_screen() {
        assert_eq!(radius(1080.0), RADIUS);
        assert_eq!(radius(2160.0), RADIUS * 2.0);
        // Never a hairline, never more lookups than the shader makes.
        assert_eq!(radius(240.0), 2.0);
        assert_eq!(radius(100_000.0), MAX_STEPS);
    }

    #[test]
    fn the_scissor_grows_by_the_ring_and_stays_on_screen() {
        let output = [1920, 1080];
        let grow = RADIUS + 2.0;
        assert_eq!(
            scissor([100.0, 200.0, 50.0, 120.0], output),
            Some([
                (100.0 - grow) as u32,
                (200.0 - grow) as u32,
                (50.0 + 2.0 * grow) as u32,
                (120.0 + 2.0 * grow) as u32,
            ])
        );
        // Cut at the edges.
        assert_eq!(
            scissor([-500.0, -500.0, 600.0, 4000.0], output),
            Some([0, 0, (100.0 + grow).ceil() as u32, 1080])
        );
        // Wholly off screen.
        assert_eq!(scissor([3000.0, 10.0, 50.0, 50.0], output), None);
    }
}
