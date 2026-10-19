//! An offscreen render of every signature skin through its own pipelines (`saber.wgsl`
//! with its signature) and its real glow and core images, on a dark ground: still, mid
//! swing, just after a clash, while it ignites and, for a chroma, worn green. Each must
//! light its blade and leave the picture's corners dark (nothing reaching the quad's
//! edge). With `SJK_SIGNATURE_PREVIEW` set to a folder, each sheet is saved there as
//! `<id>.png`; `SJK_SIGNATURES` (ids, comma-separated) renders only those.
use super::*;
use wgpu::util::DeviceExt;

const WIDTH: u32 = 240;
const HEIGHT: u32 = 300;

#[test]
#[ignore = "requires an available GPU; uses no installed game assets"]
fn every_signature_draws_its_blade_and_nothing_at_the_edge() {
    let gpu = pollster::block_on(crate::gpu_context::Context::new(None, None)).unwrap();
    let device = &gpu.device;
    let queue = &gpu.queue;
    let visible = wgpu::ShaderStages::VERTEX_FRAGMENT;
    let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: visible,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let sampler_entry = |binding| wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    };
    let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[
            crate::texture_layout_entry(0),
            crate::texture_layout_entry(1),
            crate::texture_layout_entry(3),
            sampler_entry(2),
            sampler_entry(4),
        ],
    });
    let skins_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: None,
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: visible,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: None,
        bind_group_layouts: &[
            Some(&camera_layout),
            Some(&material_layout),
            Some(&skins_layout),
        ],
        immediate_size: 0,
    });
    let pipeline =
        |module: &wgpu::ShaderModule, vertex: &str, fragment: &str, blend: wgpu::BlendState| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module,
                    entry_point: Some(vertex),
                    compilation_options: Default::default(),
                    buffers: &[Some(crate::saber::Instance::layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module,
                    entry_point: Some(fragment),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState {
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: None,
                multisample: Default::default(),
                multiview_mask: None,
                cache: None,
            })
        };
    // The blade seen side on, its 40 units filling most of the picture's height, with
    // room for 8 units of effects round it.
    let matrix = glam::camera::rh::proj::directx::orthographic(-24., 24., -28., 32., 0.1, 200.)
        * glam::camera::rh::view::look_at_mat4(
            glam::Vec3::new(0., -100., 0.),
            glam::Vec3::ZERO,
            glam::Vec3::Z,
        );
    let mut camera = matrix.to_cols_array().to_vec();
    camera.extend_from_slice(&[0., -100., 0., 0.]);
    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&camera),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    let camera_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });
    let samplers = crate::saber::Samplers::new(device);
    let save = std::env::var_os("SJK_SIGNATURE_PREVIEW").map(std::path::PathBuf::from);
    let only: Option<Vec<String>> = std::env::var("SJK_SIGNATURES").ok().map(|ids| {
        ids.split(',')
            .map(|id| format!("saber_{}", id.trim()))
            .collect()
    });
    // Every signature skin, from its built-in file.
    let skins = crate::saber_skins::LoadedSkins::of(
        SIGNATURES
            .iter()
            .map(|signature| {
                let def =
                    crate::blade_skin_file::parse(signature.id, signature.default_file()).unwrap();
                crate::saber_skins::LoadedSkin::new(
                    signature.id,
                    def,
                    &sjk_vfs::VirtualFileSystem::new(),
                )
                .unwrap()
            })
            .collect(),
        1,
    );
    let skins_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: None,
        contents: bytemuck::cast_slice(&skins.uniforms()),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let skins_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &skins_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: skins_buffer.as_entire_binding(),
        }],
    });
    for (_, skin) in skins.iter() {
        let signature = skin.signature.unwrap();
        if only
            .as_ref()
            .is_some_and(|only| !only.iter().any(|id| id == signature.id))
        {
            continue;
        }
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(signature.id),
            source: wgpu::ShaderSource::Wgsl(signature.module_source().into()),
        });
        let additive = wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: wgpu::BlendFactor::One,
                dst_factor: wgpu::BlendFactor::One,
                operation: wgpu::BlendOperation::Add,
            },
            alpha: wgpu::BlendComponent::OVER,
        };
        let glow = pipeline(&module, "vertex_main", "fragment_main", additive);
        let solid = pipeline(
            &module,
            "vertex_solid",
            "fragment_solid",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let material = crate::saber::material(
            device,
            queue,
            &material_layout,
            &samplers,
            &skin.glow,
            &skin.core,
        );
        let color = skins.color_of(signature.id).unwrap();
        let mut moments = vec![
            // Still, mid swing, just after a clash, while it ignites.
            (3.0, crate::saber_motion::STILL, color),
            (5.4, [0.9, 1_000.0, 1_000.0, 0.25], color),
            (24.0, [0.0, 0.1, 1_000.0, 1_000.0], color),
            (11.0, [0.0, 1_000.0, 0.2, 1_000.0], color),
        ];
        if crate::unlockables::is_chroma(signature.id) {
            let green = color.worn_with(crate::saber_rgb::BladeColor::from_rgb([40, 255, 60]));
            moments.push((3.0, crate::saber_motion::STILL, green));
        }
        let mut frames = Vec::new();
        for (seconds, motion, color) in moments {
            let pair = crate::saber::Instance::pair(
                crate::saber::Blade {
                    base: [0.0, 0.0, -22.0],
                    direction: [0.0, 0.0, 1.0],
                    length: 40.0,
                    radius: 3.0,
                },
                crate::saber_rgb::BladeColor::Skin(color),
            )
            .map(|i| i.with_animation(seconds, 7).with_motion(motion));
            let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(&pair),
                usage: wgpu::BufferUsages::VERTEX,
            });
            let target = device.create_texture(&wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width: WIDTH,
                    height: HEIGHT,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let view = target.create_view(&Default::default());
            let mut encoder = device.create_command_encoder(&Default::default());
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: None,
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.02,
                                g: 0.025,
                                b: 0.035,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                    multiview_mask: None,
                });
                pass.set_bind_group(0, &camera_bind, &[]);
                pass.set_bind_group(1, &material, &[]);
                pass.set_bind_group(2, &skins_bind, &[]);
                pass.set_vertex_buffer(0, vertices.slice(..));
                pass.set_pipeline(&glow);
                pass.draw(0..6, 0..2);
                pass.set_pipeline(&solid);
                pass.draw(0..6, 0..1);
            }
            queue.submit([encoder.finish()]);
            frames.push(crate::world_shot::read_back(device, queue, &target));
        }
        for frame in &frames {
            let lit = (HEIGHT / 4..HEIGHT * 3 / 4)
                .flat_map(|y| (WIDTH / 2 - 12..WIDTH / 2 + 12).map(move |x| (x, y)))
                .map(|(x, y)| {
                    frame.get_pixel(x, y).0[..3]
                        .iter()
                        .map(|&c| u32::from(c))
                        .sum::<u32>()
                })
                .max()
                .unwrap_or(0);
            assert!(lit > 120, "{}: the blade is not drawn", signature.id);
            for corner in [
                (1, 1),
                (WIDTH - 2, 1),
                (1, HEIGHT - 2),
                (WIDTH - 2, HEIGHT - 2),
            ] {
                let pixel = frame.get_pixel(corner.0, corner.1).0;
                assert!(
                    pixel[..3].iter().all(|&c| c < 24),
                    "{}: light at the picture's corner ({pixel:?})",
                    signature.id
                );
            }
        }
        if let Some(folder) = &save {
            let mut sheet = image::RgbaImage::new(WIDTH * frames.len() as u32, HEIGHT);
            for (index, frame) in frames.iter().enumerate() {
                image::imageops::overlay(&mut sheet, frame, i64::from(WIDTH) * index as i64, 0);
            }
            sheet
                .save(folder.join(format!("{}.png", signature.id)))
                .unwrap();
        }
    }
}
