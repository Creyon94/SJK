//! Shader remaps (`R_RemapShader`) applied to compiled materials mid-map.
//!
//! rd-vanilla swaps a remapped shader for its target when a surface begins drawing,
//! so the world, models and effects all follow a remap at once. Here every material
//! slot (one per BSP shader and lightmap, one per model surface) keeps the shader it
//! was built from; a remap recompiles the slot's stages from the target shader with
//! the slot's own lightmap and puts them in place of the original ones, which are
//! kept for when the remap goes away. The slot keeps its geometry and draw ranges,
//! so nothing about the map is reloaded and the draw paths are unchanged; only the
//! lists derived from the stages (draw orders, the stage table) are rebuilt.
//!
//! What stays as the original shader made it: lighting baked from it at load (lamps,
//! bounce colour, the light pre-pass set it may only leave), fog, material maps,
//! surface sprites and flares, the sky, and a surface the original shader did not
//! draw at all (a sky or a stage-less shader has no world draws to give a target).
use super::*;
use crate::world_stage::LIGHTMAP_NONE;
use sjk_shader::ShaderRemaps;

/// A slot's stages as the original shader compiled them, while a remap replaces them.
pub(super) struct Remapped {
    /// The remap target's [`sjk_shader::remap_key`].
    target: String,
    original: Look,
}

/// The fields of a [`Material`] that come from its shader's stages.
struct Look {
    stages: Vec<StagePass>,
    sort: f32,
    blended: bool,
    has_glow: bool,
    light_buffered: Option<u8>,
    light_cutout: bool,
    view_bounded: bool,
}

impl Material {
    fn swap_look(&mut self, look: &mut Look) {
        std::mem::swap(&mut self.stages, &mut look.stages);
        std::mem::swap(&mut self.sort, &mut look.sort);
        std::mem::swap(&mut self.blended, &mut look.blended);
        std::mem::swap(&mut self.has_glow, &mut look.has_glow);
        std::mem::swap(&mut self.light_buffered, &mut look.light_buffered);
        std::mem::swap(&mut self.light_cutout, &mut look.light_cutout);
        std::mem::swap(&mut self.view_bounded, &mut look.view_bounded);
    }
}

/// What a slot's original stages decide about a remapped look.
#[derive(Clone, Copy)]
struct Slot {
    flare: bool,
    light_buffered: Option<u8>,
    view_bounded: bool,
    /// Map materials carry the receiver gloss (`world_material_build.rs`); late
    /// entity materials never had one.
    gloss: bool,
}

impl Slot {
    fn of(material: &Material, lightmap: i32) -> Self {
        let gloss = match material.stages.first() {
            Some(stage) => stage
                .table
                .as_ref()
                .is_some_and(|table| table.gpu.emission[3] != 0.0),
            None => lightmap != LIGHTMAP_NONE,
        };
        Self {
            flare: material.flare,
            light_buffered: material.light_buffered,
            view_bounded: material.view_bounded,
            gloss,
        }
    }
}

/// Outcome of one [`Runtime::apply_remaps`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct Applied {
    /// Slots now drawn with a remap target.
    pub(crate) remapped: usize,
    /// Slots whose stages changed in this call (remapped or restored).
    pub(crate) changed: usize,
    /// Slots whose target failed to compile; they keep their original stages.
    pub(crate) failed: usize,
}

impl Runtime {
    /// Bring every material slot in line with `remaps`: slots whose shader is now
    /// remapped are recompiled from the target, slots no longer remapped get their
    /// original stages back. Runs only when the remap table changes, never per frame.
    pub(crate) fn apply_remaps(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        vfs: &VirtualFileSystem,
        shaders: &ShaderCatalog,
        remaps: &ShaderRemaps,
    ) -> Applied {
        let mut applied = Applied::default();
        let known = self.forge.pipeline_keys.len();
        let mut image_cache = ImageCache::with_capacity(16);
        for source in 0..self.origins.len() {
            let Some(runtime) = self
                .source_to_runtime
                .get(source)
                .copied()
                .filter(|index| *index != usize::MAX)
            else {
                continue;
            };
            let origin = &self.origins[source];
            // Surface sprites and flares are synthesised from a shader's stages and
            // keep the original shader.
            if origin.shader.starts_with('@') {
                continue;
            }
            let target = remaps.target(&origin.shader);
            let current = self.materials[runtime]
                .remapped
                .as_ref()
                .map(|remapped| remapped.target.as_str());
            if target == current {
                applied.remapped += usize::from(target.is_some());
                continue;
            }
            if let Some(mut remapped) = self.materials[runtime].remapped.take() {
                self.materials[runtime].swap_look(&mut remapped.original);
                applied.changed += 1;
            }
            let Some(target) = target else {
                continue;
            };
            let key = ViewerMaterial {
                shader: target.to_owned(),
                lightmap: origin.lightmap,
            };
            let slot = Slot::of(&self.materials[runtime], origin.lightmap);
            match self.compile_look(device, queue, vfs, shaders, &key, slot, &mut image_cache) {
                Ok(mut look) => {
                    let material = &mut self.materials[runtime];
                    material.swap_look(&mut look);
                    material.remapped = Some(Box::new(Remapped {
                        target: key.shader,
                        original: look,
                    }));
                    applied.remapped += 1;
                    applied.changed += 1;
                }
                Err(error) => {
                    applied.failed += 1;
                    crate::log::progress(format_args!(
                        "warning: shader remap {} -> {target}: {error}",
                        self.origins[source].shader
                    ));
                }
            }
        }
        if applied.changed > 0 {
            for index in known..self.forge.pipeline_keys.len() {
                let key = self.forge.pipeline_keys[index];
                self.push_pipelines(device, key);
            }
            self.refresh_after_remap(device);
        }
        applied
    }

    /// Compile `key` for a slot with its original stages, as map load and
    /// [`Runtime::append_entity_materials`] compile theirs.
    #[allow(clippy::too_many_arguments)]
    fn compile_look(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        vfs: &VirtualFileSystem,
        shaders: &ShaderCatalog,
        key: &ViewerMaterial,
        slot: Slot,
        image_cache: &mut ImageCache,
    ) -> Result<Look, Box<dyn Error>> {
        let definition = shaders.get(&key.shader);
        let undrawn = definition.is_some_and(|d| d.sky.is_some() || !d.has_color_pass());
        let lightmap = self
            .lightmaps
            .get(&key.lightmap)
            .unwrap_or(&self.forge.fallback_lightmap)
            .clone();
        let mut compiled = compile_material(
            vfs,
            shaders,
            key,
            &lightmap,
            true,
            Default::default(),
            image_cache,
            false,
        )?;
        let gloss = if definition.is_some_and(|d| {
            d.stages
                .iter()
                .any(|s| s.texture_generator == sjk_shader::TextureGenerator::Environment)
        }) {
            0.7
        } else {
            0.12
        };
        if slot.gloss {
            for stage in &mut compiled.stages {
                stage.gpu.emission[3] = gloss;
            }
        }
        let stages = if undrawn {
            Vec::new()
        } else {
            build_passes(
                device,
                queue,
                &mut self.forge,
                &self.dynamic_light_buffer,
                compiled.stages,
            )?
        };
        let opaque = compiled.sort <= SORT_OPAQUE && !slot.flare;
        let light_buffered = (opaque && stages.first().is_some_and(|stage| stage.shadow_caster))
            .then(|| match self.forge.pipeline_keys[stages[0].pipeline].cull {
                Some(wgpu::Face::Front) => 0,
                Some(wgpu::Face::Back) => 1,
                None => 2,
            });
        Ok(Look {
            sort: compiled.sort,
            blended: compiled.sort > SORT_OPAQUE,
            has_glow: glow::has_glow(stages.iter().map(|stage| stage.glow)),
            // The light pre-pass and the static lamp cache were planned for the
            // surfaces the original shader buffered: a remap may only leave that set.
            light_buffered: slot.light_buffered.and(light_buffered),
            light_cutout: opaque && stages.first().is_some_and(|stage| stage.light_cutout),
            view_bounded: slot.view_bounded && definition.is_none_or(|d| d.deforms.is_empty()),
            stages,
        })
    }

    /// Rebuild what is derived from the slots' stages: draw orders, sort keys, the
    /// stage table and the cached active set and caster runs.
    fn refresh_after_remap(&mut self, device: &wgpu::Device) {
        for (source, &runtime) in self.source_to_runtime.iter().enumerate() {
            if let Some(material) = self.materials.get(runtime) {
                self.source_order[source] = (
                    material.sort,
                    material.stages.first().map_or(0, |stage| stage.pipeline),
                );
            }
        }
        self.opaque_order = build_draw_order(&self.materials, false);
        self.blended_order = build_draw_order(&self.materials, true);
        self.glow_order = glow::order(&self.materials, &self.opaque_order, &self.blended_order);
        if let Some(table) = &mut self.stage_table {
            table.rebuild(device, &self.forge, &self.materials);
        }
        *self.active.get_mut() = Default::default();
        self.caster_runs = std::cell::OnceCell::new();
    }

    /// The shader each slot is drawn with now, for `listRemaps`: `(original, target)`
    /// for every remapped slot, once per distinct pair.
    pub(crate) fn remapped_slots(&self) -> Vec<(&str, &str)> {
        let mut pairs: Vec<(&str, &str)> = self
            .source_to_runtime
            .iter()
            .enumerate()
            .filter_map(|(source, &runtime)| {
                let remapped = self.materials.get(runtime)?.remapped.as_ref()?;
                Some((
                    self.origins.get(source)?.shader.as_str(),
                    remapped.target.as_str(),
                ))
            })
            .collect();
        pairs.sort_unstable();
        pairs.dedup();
        pairs
    }
}
