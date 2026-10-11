//! The player card outline's mask ([`crate::card_outline`]): the inspected player's
//! body drawn again with the scene's vertex path, bind groups and instance, into a
//! one-channel image where it is in view. Like the glow pass it reads the scene's
//! finished depth and never writes it; a slight bias towards the eye keeps the body
//! from losing against its own depth.
use super::*;

/// Pull the mask a little towards the eye: the body's own depth never rejects it, and
/// nothing thinner than the bias in front of the body is missed.
const BIAS: wgpu::DepthBiasState = wgpu::DepthBiasState {
    constant: -16,
    slope_scale: -1.0,
    clamp: 0.0,
};

impl Runtime {
    /// The mask pipeline for the mask material's stage key `index`, compiled on first use.
    fn card_outline_pipeline(&self, index: usize) -> &wgpu::RenderPipeline {
        self.card_outline_mask.get_or_init(|| {
            let key = self.forge.pipeline_keys[index];
            let (layout, shader) = self.forge.program_for(key);
            self.forge
                .device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("SJK card outline mask"),
                    layout: Some(layout),
                    vertex: wgpu::VertexState {
                        module: shader,
                        entry_point: Some("entity_vertex_main"),
                        compilation_options: Default::default(),
                        buffers: &[
                            Some(crate::GpuVertex::layout()),
                            Some(crate::ActorInstance::layout()),
                        ],
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader,
                        entry_point: Some("card_outline_mask_fragment"),
                        compilation_options: Default::default(),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: crate::card_outline::MASK_FORMAT,
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                    }),
                    // Every face: the mask is the shape the body covers, two-sided
                    // surfaces (hair, cloth) included.
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: Some(wgpu::DepthStencilState {
                        format: crate::DepthTarget::FORMAT,
                        depth_write_enabled: Some(false),
                        depth_compare: Some(wgpu::CompareFunction::LessEqual),
                        stencil: Default::default(),
                        bias: BIAS,
                    }),
                    multisample: Default::default(),
                    multiview_mask: None,
                    cache: None,
                })
        })
    }

    /// Draw the outline's `draws` (the mask material over the body's surfaces) into a
    /// pass whose colour target is the mask and whose depth is the scene's, read-only.
    pub(crate) fn draw_card_outline_mask<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        frame: &FrameDraw<'pass>,
        draws: &'pass [crate::entity_materials::Draw],
    ) {
        pass.set_bind_group(0, frame.camera, &[]);
        pass.set_bind_group(2, &self.forge.geometry, &[]);
        if let Some(sun) = &self.forge.model_sun {
            pass.set_bind_group(3, sun.binding(true), &[]);
        }
        pass.set_vertex_buffer(0, frame.vertices.slice(..));
        pass.set_vertex_buffer(1, frame.instances.slice(..));
        pass.set_index_buffer(frame.indices.slice(..), wgpu::IndexFormat::Uint32);
        for draw in draws {
            let Some(material) = self
                .source_to_runtime
                .get(draw.material)
                .copied()
                .filter(|index| *index != usize::MAX)
                .and_then(|index| self.materials.get(index))
            else {
                continue;
            };
            let Some(stage) = material.stages.first() else {
                continue;
            };
            pass.set_pipeline(self.card_outline_pipeline(stage.pipeline));
            pass.set_bind_group(1, stage.color_group(), &[]);
            pass.draw_indexed(draw.indices.clone(), 0, draw.instances.clone());
        }
    }
}
