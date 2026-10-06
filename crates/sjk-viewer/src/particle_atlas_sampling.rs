//! Sample only the particle shader stages consumed by the current draw.
use crate::*;

impl ParticleAtlas {
    /// Sample the first authored stage, or the atlas fallback for an unknown shader.
    pub(crate) fn first_layer(&self, shader: &str, age_seconds: f32) -> ParticleLayerSample {
        self.stages(shader)
            .and_then(|animations| animations.first())
            .map(|animation| self.sample_animation(animation, age_seconds))
            .unwrap_or(ParticleLayerSample {
                uv_rect: self.fallback,
                blend: ParticleBlend::Add,
                rgb: 1.0,
                alpha: 1.0,
                uv_transform: [1.0, 1.0, 0.0, 0.0],
                glow: false,
            })
    }

    /// Evaluate one stage's original frame, waveform and texture-coordinate rules.
    pub(crate) fn sample_animation(
        &self,
        animation: &ParticleAtlasAnimation,
        age_seconds: f32,
    ) -> ParticleLayerSample {
        let age_seconds = age_seconds - animation.time_offset;
        if animation.frames.len() == 1 || animation.frequency <= 0.0 {
            return ParticleLayerSample {
                uv_rect: animation.frames.first().copied().unwrap_or(self.fallback),
                blend: animation.blend,
                rgb: effect_wave::evaluate(animation.rgb_wave.as_ref(), age_seconds),
                alpha: effect_wave::evaluate(animation.alpha_wave.as_ref(), age_seconds),
                uv_transform: effect_texcoords::sample(
                    animation.tc_scale,
                    animation.tc_scroll,
                    age_seconds,
                ),
                glow: animation.glow,
            };
        }
        let raw = (age_seconds.max(0.0) * animation.frequency).floor() as usize;
        let frame = if animation.one_shot {
            raw.min(animation.frames.len() - 1)
        } else {
            raw % animation.frames.len()
        };
        ParticleLayerSample {
            uv_rect: animation.frames[frame],
            blend: animation.blend,
            rgb: effect_wave::evaluate(animation.rgb_wave.as_ref(), age_seconds),
            alpha: effect_wave::evaluate(animation.alpha_wave.as_ref(), age_seconds),
            uv_transform: effect_texcoords::sample(
                animation.tc_scale,
                animation.tc_scroll,
                age_seconds,
            ),
            glow: animation.glow,
        }
    }

    /// Retain a borrowed stage selection without filling an eight-stage scratch array.
    pub(crate) fn layers_for(
        &self,
        shader: &str,
        age_seconds: f32,
    ) -> effect_runtime::ParticleLayerSamples<'_> {
        effect_runtime::ParticleLayerSamples::new(
            self,
            self.stages(shader).map_or(&[], Vec::as_slice),
            age_seconds,
        )
    }

    /// A shader's stages. Shader names are case-insensitive (the atlas keys them in
    /// lower case); a mixed-case name such as `gfx/effects/saberFlare` used to miss
    /// and draw the fallback spark, scaled to the clash flare's full-screen size.
    fn stages(&self, shader: &str) -> Option<&Vec<ParticleAtlasAnimation>> {
        if shader.bytes().any(|byte| byte.is_ascii_uppercase()) {
            self.animations.get(shader.to_ascii_lowercase().as_str())
        } else {
            self.animations.get(shader)
        }
    }
}
