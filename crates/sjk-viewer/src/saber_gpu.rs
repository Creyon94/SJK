//! Production WGPU resources for saber blades and motion trails.

use crate::saber::{self, Instance};
use crate::saber_skins::{LoadedSkins, MAX_SKINS, SkinUniform};
use crate::saber_trail::SegmentPool;
use crate::{DepthTarget, texture_layout_entry};
use sjk_shader::ShaderCatalog;
use sjk_vfs::VirtualFileSystem;
use std::error::Error;
use std::ops::Range;
use std::sync::{Arc, OnceLock};

#[path = "saber_glow_integral.rs"]
pub(crate) mod glow_integral;

pub(crate) struct Runtime {
    pipeline: wgpu::RenderPipeline,
    black_pipeline: wgpu::RenderPipeline,
    /// The glow capsules alone, for the dynamic glow image: the retail `*_glow` blade
    /// shaders carry `glow`, the `*_line` cores do not (`sabers.shader`).
    glow_pipeline: wgpu::RenderPipeline,
    materials: Vec<wgpu::BindGroup>,
    /// What a blade-skin slot's pair is made with when a skin loads.
    texture_layout: wgpu::BindGroupLayout,
    samplers: saber::Samplers,
    /// The loaded blade skins' parameters (`saber.wgsl` `skins`), written only when
    /// skins load.
    skins_buffer: wgpu::Buffer,
    skins: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    trails: crate::saber_trail_gpu::Runtime,
    /// The classic model preview's blades, kept apart from the world's.
    preview_buffer: wgpu::Buffer,
    preview_ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    /// The Collection's blade pictures (`blade_pictures.rs`), kept apart from the others.
    pictures_buffer: wgpu::Buffer,
    pictures_ranges: [Range<u32>; crate::saber_rgb::MATERIAL_COUNT],
    /// Skins' heat haze (`saber.wgsl` `fragment_haze`): drawn first into the effect layer,
    /// replacing its pixels with the scene before effects, bent.
    haze_pipeline: wgpu::RenderPipeline,
    haze_layout: wgpu::BindGroupLayout,
    haze_sampler: wgpu::Sampler,
    /// The effect layer's scene image bound for the haze, made again only when the layer
    /// is rebuilt (a resize).
    haze_source: std::cell::RefCell<Option<(wgpu::TextureView, wgpu::BindGroup)>>,
    /// Whether a loaded skin has haze at all.
    any_haze: bool,
    /// Skins' black holes: their lens (`saber.wgsl` `fragment_lens`), drawn with the haze
    /// and blended over it, and their horizons (`fragment_shade`), blended black over
    /// the blades' glow after it.
    lens_pipeline: wgpu::RenderPipeline,
    shade_pipeline: wgpu::RenderPipeline,
    /// Whether a loaded skin has black holes at all.
    any_holes: bool,
    /// Skins' solid parts (`saber.wgsl` `fragment_solid`: ink and chrome), blended over the
    /// blades just drawn, and whether a loaded skin has any.
    solid_pipeline: wgpu::RenderPipeline,
    any_solids: bool,
    /// What the signature skins' own pipelines are made with (`blade_signatures.rs`).
    builder: SignatureBuilder,
    /// The loaded skins' signatures by skin number, and their pipelines once built.
    signed: Vec<Option<Signed>>,
}

/// A loaded skin's signature shader and its pipelines: built from its own module off the
/// render thread, so the frame never waits. Every loaded signature is built in the
/// background as soon as the skins load, several at once (so a blade worn later is ready),
/// and one a blade wears (or the preview shows) before its turn comes is built at once on a
/// thread of its own, its three pipelines side by side. Until then its blades are drawn by
/// the shared pipelines, from the skin's file.
#[derive(Clone)]
struct Signed {
    signature: &'static crate::blade_signatures::Signature,
    built: Arc<OnceLock<Option<SignaturePipelines>>>,
    started: Arc<std::sync::atomic::AtomicBool>,
}

impl Signed {
    /// Claim the build: true the first time only.
    fn claim(&self) -> bool {
        !self.started.swap(true, std::sync::atomic::Ordering::AcqRel)
    }

    /// Build it here (on the calling thread) if nobody has claimed it.
    fn build_if_unclaimed(&self, builder: &SignatureBuilder) {
        if self.claim() {
            let _ = self.built.set(builder.build(self.signature));
        }
    }
}

/// A signature skin's glow (`fragment_main`), bloom (`fragment_glow`) and solid
/// (`fragment_solid`) pipelines.
struct SignaturePipelines {
    main: wgpu::RenderPipeline,
    glow: wgpu::RenderPipeline,
    solid: wgpu::RenderPipeline,
}

/// The passes a signature skin has pipelines of its own for.
#[derive(Clone, Copy)]
enum Pass {
    Main,
    Glow,
    Solid,
}

/// The device, layout and target format the blade pipelines are made with, kept to build
/// the signature skins' own.
#[derive(Clone)]
struct SignatureBuilder {
    device: wgpu::Device,
    layout: wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
}

impl SignatureBuilder {
    /// A blade pipeline: `vertex` and `fragment` of `module`, depth-tested, blended so.
    fn pipeline(
        &self,
        label: &str,
        module: &wgpu::ShaderModule,
        vertex: &str,
        fragment: &str,
        blend: Option<wgpu::BlendState>,
    ) -> wgpu::RenderPipeline {
        blade_pipeline(
            &self.device,
            &self.layout,
            self.format,
            label,
            module,
            vertex,
            fragment,
            blend,
        )
    }

    /// `signature`'s pipelines; `None` (logged) when its module does not build. Its three
    /// pipelines are built at once, each on a thread of its own (Creyon's round 19: a blade's
    /// effects took several seconds to come in, in the preview and when worn), each thread
    /// catching its own errors (error scopes are the thread's own).
    fn build(&self, signature: &crate::blade_signatures::Signature) -> Option<SignaturePipelines> {
        let started = std::time::Instant::now();
        let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
        let module = self
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some(signature.id),
                source: wgpu::ShaderSource::Wgsl(signature.module_source().into()),
            });
        let mut failed = pollster::block_on(scope.pop()).map(|error| error.to_string());
        let checked = |vertex: &str, fragment: &str, blend: wgpu::BlendState| {
            let scope = self.device.push_error_scope(wgpu::ErrorFilter::Validation);
            let pipeline = self.pipeline(signature.id, &module, vertex, fragment, Some(blend));
            (
                pipeline,
                pollster::block_on(scope.pop()).map(|error| error.to_string()),
            )
        };
        let (main, glow, solid) = std::thread::scope(|threads| {
            let glow = threads
                .spawn(|| checked("vertex_main", "fragment_glow", wgpu::BlendState::ADDITIVE));
            let solid = threads.spawn(|| {
                checked(
                    "vertex_solid",
                    "fragment_solid",
                    wgpu::BlendState::ALPHA_BLENDING,
                )
            });
            let main = checked("vertex_main", "fragment_main", wgpu::BlendState::ADDITIVE);
            (main, glow.join(), solid.join())
        });
        let (Ok(glow), Ok(solid)) = (glow, solid) else {
            crate::log::progress(format_args!(
                "blade skins: {}'s signature shader: a build thread failed",
                signature.id
            ));
            return None;
        };
        for (_, error) in [&main, &glow, &solid] {
            if failed.is_none() {
                failed.clone_from(error);
            }
        }
        if let Some(error) = failed {
            crate::log::progress(format_args!(
                "blade skins: {}'s signature shader: {error}",
                signature.id
            ));
            return None;
        }
        let pipelines = SignaturePipelines {
            main: main.0,
            glow: glow.0,
            solid: solid.0,
        };
        crate::log::progress(format_args!(
            "blade skins: {}'s signature shader built in {} ms",
            signature.id,
            started.elapsed().as_millis()
        ));
        Some(pipelines)
    }
}

/// A blade pipeline over the instances: `vertex` and `fragment` of `module`, depth-tested
/// against the scene without writing depth, blended with `blend`.
#[allow(clippy::too_many_arguments)]
fn blade_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    label: &str,
    module: &wgpu::ShaderModule,
    vertex: &str,
    fragment: &str,
    blend: Option<wgpu::BlendState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some(vertex),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(Instance::layout())],
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(fragment),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend,
                // Only what covers (solid parts, black holes) writes alpha, so a target
                // cleared transparent keeps their coverage apart from the added light (the
                // model preview lays them over its backdrop by it, `preview.wgsl`).
                write_mask: if blend == Some(wgpu::BlendState::ALPHA_BLENDING) {
                    wgpu::ColorWrites::ALL
                } else {
                    wgpu::ColorWrites::COLOR
                },
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
}

/// Blade instances the model preview can draw: two sabers of eight blades, each its glow,
/// core and echo and a skin's glints.
pub(crate) const PREVIEW_BLADES: usize =
    2 * 8 * (3 + crate::blade_skin_file::extras::MAX_GLINTS as usize);

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
            label: Some("SJK saber shader"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        };
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber texture layout"),
            entries: &[
                texture_layout_entry(0),
                texture_layout_entry(1),
                texture_layout_entry(3),
                sampler_entry(2),
                sampler_entry(4),
            ],
        });
        let samplers = saber::Samplers::new(device);
        let materials =
            saber::create_materials(device, queue, vfs, shaders, &texture_layout, &samplers)?;
        let skins_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber skins layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let skins_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK saber skins"),
            size: std::mem::size_of::<[SkinUniform; MAX_SKINS]>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let skins = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK saber skins"),
            layout: &skins_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: skins_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK saber pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&texture_layout),
                Some(&skins_layout),
            ],
            immediate_size: 0,
        });
        let haze_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("SJK saber haze scene layout"),
            entries: &[texture_layout_entry(0), sampler_entry(1)],
        });
        let haze_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("SJK saber haze pipeline layout"),
            bind_group_layouts: &[
                Some(camera_layout),
                Some(&texture_layout),
                Some(&skins_layout),
                Some(&haze_layout),
            ],
            immediate_size: 0,
        });
        let create = |fragment| {
            blade_pipeline(
                device,
                &layout,
                format,
                "SJK saber pipeline",
                &shader,
                "vertex_main",
                fragment,
                Some(wgpu::BlendState::ADDITIVE),
            )
        };
        let pipeline = create("fragment_main");
        let glow_pipeline = create("fragment_glow");
        // The haze replaces what it covers (the layer still holds the scene there, as no
        // effect has drawn yet), depth-tested so a wall before the blade stays sharp; a
        // black hole's lens and horizon blend over what is there.
        let special = |label, layout, vertex, fragment, blend| {
            blade_pipeline(
                device, layout, format, label, &shader, vertex, fragment, blend,
            )
        };
        let haze_pipeline = special(
            "SJK saber haze pipeline",
            &haze_pipeline_layout,
            "vertex_haze",
            "fragment_haze",
            None,
        );
        let lens_pipeline = special(
            "SJK saber black hole lens pipeline",
            &haze_pipeline_layout,
            "vertex_hole",
            "fragment_lens",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let shade_pipeline = special(
            "SJK saber black hole horizon pipeline",
            &layout,
            "vertex_hole",
            "fragment_shade",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let solid_pipeline = special(
            "SJK saber solid pipeline",
            &layout,
            "vertex_solid",
            "fragment_solid",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let black_pipeline = special(
            "SJK JA+ black saber pipeline",
            &layout,
            "vertex_main",
            "fragment_black",
            Some(wgpu::BlendState::ALPHA_BLENDING),
        );
        let haze_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("SJK saber haze sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK saber instances"),
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
        let pictures_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SJK blade picture saber instances"),
            size: (crate::blade_pictures::BLADES * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Ok(Self {
            pipeline,
            black_pipeline,
            glow_pipeline,
            materials,
            texture_layout,
            samplers,
            skins_buffer,
            skins,
            instance_buffer,
            preview_buffer,
            preview_ranges: std::array::from_fn(|_| 0..0),
            pictures_buffer,
            pictures_ranges: std::array::from_fn(|_| 0..0),
            ranges: std::array::from_fn(|_| 0..0),
            haze_pipeline,
            haze_layout,
            haze_sampler,
            haze_source: std::cell::RefCell::new(None),
            any_haze: false,
            lens_pipeline,
            shade_pipeline,
            any_holes: false,
            solid_pipeline,
            any_solids: false,
            builder: SignatureBuilder {
                device: device.clone(),
                layout,
                format,
            },
            signed: Vec::new(),
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

    /// Take the loaded blade skins: their parameters into the uniform array and their
    /// glow/core pairs into their material slots. Only when skins load, never per frame.
    pub(crate) fn upload_skins(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        skins: &LoadedSkins,
    ) {
        queue.write_buffer(
            &self.skins_buffer,
            0,
            bytemuck::cast_slice(&skins.uniforms()),
        );
        self.any_haze = skins.iter().any(|(_, skin)| skin.shown().haze.is_some());
        self.any_holes = skins.iter().any(|(_, skin)| skin.shown().holes.is_some());
        self.any_solids = skins.iter().any(|(_, skin)| {
            skin.shown().solids().any()
                || skin.design_shown().is_some_and(|design| design.solid)
                || skin
                    .signature_shown()
                    .is_some_and(|signature| signature.solid())
        });
        // Pipelines already built for a signature at the same number are kept.
        let mut signed: Vec<Option<Signed>> = (0..MAX_SKINS).map(|_| None).collect();
        for (index, skin) in skins.iter() {
            let Some(signature) = skin.signature_shown() else {
                continue;
            };
            let kept = self
                .signed
                .get_mut(usize::from(index))
                .and_then(Option::take)
                .filter(|old| std::ptr::eq(old.signature, signature));
            signed[usize::from(index)] = Some(kept.unwrap_or(Signed {
                signature,
                built: Arc::default(),
                started: Arc::default(),
            }));
        }
        self.signed = signed;
        // Build every loaded signature in the background, several at once (round 19: one
        // after another, the last came in long after the game started), on up to four
        // threads, leaving a core to the game.
        let pending: Arc<Vec<Signed>> = Arc::new(self.signed.iter().flatten().cloned().collect());
        if !pending.is_empty() {
            let workers = std::thread::available_parallelism()
                .map_or(1, |cores| cores.get().saturating_sub(1))
                .clamp(1, 4)
                .min(pending.len());
            let next = Arc::new(std::sync::atomic::AtomicUsize::new(0));
            for _ in 0..workers {
                let builder = self.builder.clone();
                let pending = pending.clone();
                let next = next.clone();
                let spawned = std::thread::Builder::new()
                    .name("sjk-blade-signatures".to_owned())
                    .spawn(move || {
                        loop {
                            let index = next.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                            let Some(signed) = pending.get(index) else {
                                break;
                            };
                            signed.build_if_unclaimed(&builder);
                        }
                    });
                if let Err(why) = spawned {
                    crate::log::progress(format_args!(
                        "blade skins: signature shaders: no thread: {why}"
                    ));
                    break;
                }
            }
        }
        for (index, skin) in skins.iter() {
            self.materials[crate::saber_rgb::SKIN_MATERIAL as usize + usize::from(index)] =
                saber::material(
                    device,
                    queue,
                    &self.texture_layout,
                    &self.samplers,
                    &skin.glow,
                    &skin.core,
                );
        }
    }

    pub(crate) fn prepare(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
        trails: &mut SegmentPool,
        now: i64,
    ) {
        self.ranges = saber::material_ranges(blades);
        self.build_signatures(&self.ranges);
        if !blades.is_empty() {
            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(blades));
        }
        self.trails.prepare(queue, trails, now);
    }

    /// Start building, each on a thread of its own, the pipelines of every signature skin
    /// `ranges` draws whose build has not begun.
    fn build_signatures(&self, ranges: &[Range<u32>]) {
        let skins = crate::saber_rgb::SKIN_MATERIAL as usize;
        for (signed, range) in self.signed.iter().zip(&ranges[skins..]) {
            let Some(signed) = signed else {
                continue;
            };
            if range.is_empty() || !signed.claim() {
                continue;
            }
            let builder = self.builder.clone();
            let built = signed.built.clone();
            let signature = signed.signature;
            let spawned = std::thread::Builder::new()
                .name("sjk-blade-signature".to_owned())
                .spawn(move || {
                    let _ = built.set(builder.build(signature));
                });
            if let Err(why) = spawned {
                crate::log::progress(format_args!(
                    "blade skins: {}'s signature shader: no thread: {why}",
                    signature.id
                ));
            }
        }
    }

    /// The pipeline drawing material `material` in `pass`: a signature skin's own once
    /// built, else `shared`.
    fn pick<'a>(
        &'a self,
        material: usize,
        pass: Pass,
        shared: &'a wgpu::RenderPipeline,
    ) -> &'a wgpu::RenderPipeline {
        let own = material
            .checked_sub(crate::saber_rgb::SKIN_MATERIAL as usize)
            .and_then(|index| self.signed.get(index)?.as_ref()?.built.get()?.as_ref());
        match (own, pass) {
            (Some(own), Pass::Main) => &own.main,
            (Some(own), Pass::Glow) => &own.glow,
            (Some(own), Pass::Solid) => &own.solid,
            (None, _) => shared,
        }
    }

    pub(crate) fn draw<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        self.trails.draw(pass, camera, true);
        self.draw_blades(pass, camera, Pass::Main);
        self.draw_horizons(pass, camera, &self.instance_buffer, &self.ranges, true);
    }

    /// Whether this frame's blades may bend the scene with heat haze or a black hole's
    /// lens: a loaded skin has one and a skinned blade is drawn (the vertex stage folds
    /// away every other one).
    pub(crate) fn has_haze(&self) -> bool {
        (self.any_haze || self.any_holes) && skinned(&self.ranges)
    }

    /// The black holes' horizons and the skins' solid parts (ink, chrome) over the blades
    /// just drawn from `buffer`.
    fn draw_horizons<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        buffer: &'pass wgpu::Buffer,
        ranges: &[Range<u32>],
        black_blades: bool,
    ) {
        let black = crate::saber_rgb::BLACK_MATERIAL as usize;
        if black_blades && !ranges[black].is_empty() {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |_| &self.black_pipeline,
                &self.materials[black..],
                buffer,
                &ranges[black..],
            );
        }
        if self.any_holes && skinned(ranges) {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |_| &self.shade_pipeline,
                &self.materials,
                buffer,
                ranges,
            );
        }
        if self.any_solids && skinned(ranges) {
            draw_instances(
                pass,
                camera,
                &self.skins,
                |material| self.pick(material, Pass::Solid, &self.solid_pipeline),
                &self.materials,
                buffer,
                ranges,
            );
        }
    }

    /// Draw the skins' heat haze into the effect layer whose scene image is `scene`
    /// (`effect_layer.rs` `original`), before any other effect: each pixel it covers is
    /// the scene there, bent.
    pub(crate) fn draw_haze(
        &self,
        device: &wgpu::Device,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &wgpu::BindGroup,
        scene: &wgpu::TextureView,
    ) {
        if !self.has_haze() {
            return;
        }
        let bind = {
            let mut source = self.haze_source.borrow_mut();
            match source.as_ref() {
                Some((view, bind)) if view == scene => bind.clone(),
                _ => {
                    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
                        label: Some("SJK saber haze scene"),
                        layout: &self.haze_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(scene),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::Sampler(&self.haze_sampler),
                            },
                        ],
                    });
                    *source = Some((scene.clone(), bind.clone()));
                    bind
                }
            }
        };
        // The haze first, then the lenses over it: each in its own run, as the instances
        // of a material are in no set order.
        let pipelines = [
            (self.any_haze, &self.haze_pipeline),
            (self.any_holes, &self.lens_pipeline),
        ];
        for (_, pipeline) in pipelines.into_iter().filter(|(any, _)| *any) {
            pass.set_pipeline(pipeline);
            pass.set_bind_group(0, camera, &[]);
            pass.set_bind_group(2, &self.skins, &[]);
            pass.set_bind_group(3, &bind, &[]);
            pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
            let skins = crate::saber_rgb::SKIN_MATERIAL as usize;
            for (material, instances) in self
                .materials
                .iter()
                .zip(self.ranges.iter().cloned())
                .skip(skins)
            {
                if instances.is_empty() {
                    continue;
                }
                pass.set_bind_group(1, material, &[]);
                pass.draw(0..6, instances);
            }
        }
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
        self.build_signatures(&self.preview_ranges);
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
            &self.skins,
            |material| self.pick(material, Pass::Main, &self.pipeline),
            &self.materials,
            &self.preview_buffer,
            &self.preview_ranges,
        );
        self.draw_horizons(
            pass,
            camera,
            &self.preview_buffer,
            &self.preview_ranges,
            true,
        );
    }

    /// Upload the Collection's blade pictures' blades (at most
    /// [`crate::blade_pictures::BLADES`]), sorted by material.
    pub(crate) fn prepare_pictures(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        blades: &mut [Instance],
    ) {
        let count = blades.len().min(crate::blade_pictures::BLADES);
        let blades = &mut blades[..count];
        self.pictures_ranges = saber::material_ranges(blades);
        self.build_signatures(&self.pictures_ranges);
        if !blades.is_empty() {
            queue.write_buffer(&self.pictures_buffer, 0, bytemuck::cast_slice(blades));
        }
    }

    /// Draw the blade pictures' blades with the pictures' `camera`.
    pub(crate) fn draw_pictures<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
    ) {
        draw_instances(
            pass,
            camera,
            &self.skins,
            |material| self.pick(material, Pass::Main, &self.pipeline),
            &self.materials,
            &self.pictures_buffer,
            &self.pictures_ranges,
        );
        self.draw_horizons(
            pass,
            camera,
            &self.pictures_buffer,
            &self.pictures_ranges,
            true,
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
            self.trails.draw(pass, camera, false);
        }
        self.draw_blades(pass, camera, Pass::Glow);
        // A hole in front of the blade hides its glow there too.
        self.draw_horizons(pass, camera, &self.instance_buffer, &self.ranges, false);
    }

    /// Whether the glow image gets any saber this frame.
    pub(crate) fn has_glow(&self, blades_only: bool) -> bool {
        (!blades_only && self.trails.has_draws()) || self.ranges.iter().any(|r| !r.is_empty())
    }

    fn draw_blades<'pass>(
        &'pass self,
        pass: &mut wgpu::RenderPass<'pass>,
        camera: &'pass wgpu::BindGroup,
        kind: Pass,
    ) {
        let shared = match kind {
            Pass::Glow => &self.glow_pipeline,
            _ => &self.pipeline,
        };
        draw_instances(
            pass,
            camera,
            &self.skins,
            |material| self.pick(material, kind, shared),
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

/// Whether `ranges` holds a skinned blade.
fn skinned(ranges: &[Range<u32>]) -> bool {
    ranges[crate::saber_rgb::SKIN_MATERIAL as usize..crate::saber_rgb::BLACK_MATERIAL as usize]
        .iter()
        .any(|range| !range.is_empty())
}

/// Draw the blade instances of `buffer`, a run per material, each with the pipeline
/// `pipeline` picks for its place in `materials` (a signature skin's may be its own).
fn draw_instances<'pass>(
    pass: &mut wgpu::RenderPass<'pass>,
    camera: &'pass wgpu::BindGroup,
    skins: &'pass wgpu::BindGroup,
    pipeline: impl Fn(usize) -> &'pass wgpu::RenderPipeline,
    materials: &'pass [wgpu::BindGroup],
    buffer: &'pass wgpu::Buffer,
    ranges: &[Range<u32>],
) {
    if ranges.iter().all(Range::is_empty) {
        return;
    }
    // Materials offset from the first (the black blades') pick by their own number.
    let first = crate::saber_rgb::MATERIAL_COUNT - materials.len();
    let mut current: Option<&wgpu::RenderPipeline> = None;
    for (index, (material, instances)) in materials.iter().zip(ranges.iter().cloned()).enumerate() {
        if instances.is_empty() {
            continue;
        }
        let chosen = pipeline(first + index);
        if current.is_none_or(|current| !std::ptr::eq(current, chosen)) {
            pass.set_pipeline(chosen);
            if current.is_none() {
                pass.set_bind_group(0, camera, &[]);
                pass.set_bind_group(2, skins, &[]);
                pass.set_vertex_buffer(0, buffer.slice(..));
            }
            current = Some(chosen);
        }
        pass.set_bind_group(1, material, &[]);
        pass.draw(0..6, instances);
    }
}
