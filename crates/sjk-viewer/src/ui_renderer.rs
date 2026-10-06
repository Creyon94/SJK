//! Viewer-side GPU/text adapter for renderer-neutral UI draw commands.

use crate::text::{self, TextVertex, UiFont};
use bytemuck::{Pod, Zeroable};
use sjk_ui::{Color, DrawCommand, DrawList, FontWeight, Rect, TextAlign, TextId};

pub(crate) mod art;
mod emblem;
mod icons;
mod levelshot;
use art::{ArtTextures, Run, Source};
use emblem::EmblemTextures;
use icons::IconAtlas;
pub(crate) use icons::{
    ATLAS_CELLS, BANNER_SIZE, BANNER_TEXTURE, BIND_ICON_CELLS, BIND_ICON_FIRST, FORCE_ICON_CELLS,
    FORCE_ICON_FIRST, FORCE_WHEEL_ICON_CELLS, FORCE_WHEEL_ICON_FIRST, ICON_CELLS, ICON_SIZE,
    PART_ICON_CELLS, PART_ICON_FIRST, SCOREBOARD_ICON_CELLS,
};
pub(crate) use levelshot::LEVELSHOT_TEXTURE;

/// `TexturedQuad` texture naming the classic profile's model preview
/// (`menu_stage::preview`), bound by [`ShapeRenderer::set_preview`].
pub(crate) const PREVIEW_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(u32::MAX - 2);
/// `TexturedQuad` texture naming the settings' HUD picker preview, uploaded by
/// [`ShapeRenderer::upload_hud_preview`].
pub(crate) const HUD_PREVIEW_TEXTURE: sjk_ui::TextureId = sjk_ui::TextureId(u32::MAX - 3);
use levelshot::LevelshotTexture;

/// Main-menu wordmark: the Jedi Knight saber emblem laid horizontal, white
/// on transparent, tinted by the player's accent at draw time.
const MENU_WORDMARK: &[u8] = include_bytes!("../assets/menu/jk-wordmark.png");

/// Shape vertices one frame may draw (six per quad), across every layer.
const MAX_SHAPE_VERTICES: usize = 16_384;

/// `parameters.y` value that selects the arc shader path (`ui_shapes.wgsl`).
const ARC_MODE: f32 = 3.0;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(super) struct ShapeVertex {
    position: [f32; 2],
    local: [f32; 2],
    size: [f32; 2],
    start_color: [f32; 4],
    end_color: [f32; 4],
    parameters: [f32; 2],
    uv: [f32; 2],
}

impl ShapeVertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 7] = wgpu::vertex_attr_array![
        0 => Float32x2,
        1 => Float32x2,
        2 => Float32x2,
        3 => Float32x4,
        4 => Float32x4,
        5 => Float32x2,
        6 => Float32x2
    ];

    fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

/// Colour adds as light (`src * alpha + dst`) and the target's alpha is
/// kept: the menu emblem's glow layers, whose black adds nothing.
const ADDITIVE_BLENDING: wgpu::BlendState = wgpu::BlendState {
    color: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::SrcAlpha,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
    alpha: wgpu::BlendComponent {
        src_factor: wgpu::BlendFactor::Zero,
        dst_factor: wgpu::BlendFactor::One,
        operation: wgpu::BlendOperation::Add,
    },
};

/// Texture switches (bind-group runs) the renderer makes for `lists` with every
/// texture loaded, as [`ShapeRenderer::prepare_layers`] counts them against
/// [`art::MAX_RUNS`]; for screens' tests.
#[cfg(test)]
pub(crate) fn texture_switches(lists: &[&DrawList]) -> usize {
    let mut runs = vec![Run {
        start: 0,
        source: Source::Atlas,
    }];
    let mut vertices = 0;
    for command in lists.iter().flat_map(|list| list.commands()) {
        let texture = match command {
            DrawCommand::TexturedQuad { texture, .. }
            | DrawCommand::TexturedQuadUv { texture, .. } => *texture,
            _ => continue,
        };
        let source = match crate::menu::art::ArtPiece::from_texture(texture) {
            Some(piece) => Source::Art(piece),
            None if texture == LEVELSHOT_TEXTURE => Source::Levelshot,
            None if texture == HUD_PREVIEW_TEXTURE => Source::HudPreview,
            None if texture == PREVIEW_TEXTURE => Source::Preview,
            None => crate::menu::emblem::EmblemLayer::from_texture(texture)
                .map_or(Source::Atlas, Source::Emblem),
        };
        if runs.len() < art::MAX_RUNS {
            art::switch(&mut runs, vertices, source);
        } else if runs.last().is_some_and(|run| run.source != source) {
            runs.push(Run {
                start: vertices as u32,
                source,
            });
        }
        vertices += 6;
    }
    runs.len()
}

/// Fixed-capacity WGPU shape renderer. GPU ownership never leaks into `sjk-ui`.
pub(crate) struct ShapeRenderer {
    pipeline: wgpu::RenderPipeline,
    /// The same shapes blended additively ([`ADDITIVE_BLENDING`]), for the
    /// runs of the menu emblem's glow layers.
    additive_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertices: Vec<ShapeVertex>,
    icons: IconAtlas,
    /// Bind-group runs of `vertices`, in draw order.
    runs: Vec<Run>,
    /// Layout the icon atlas and the classic menu art are bound with.
    texture_layout: wgpu::BindGroupLayout,
    /// The player's retail menu artwork, one texture per piece.
    art: ArtTextures,
    /// SJK's menu emblem, one texture per layer.
    emblem: EmblemTextures,
    /// The current map preview at its own resolution.
    levelshot: LevelshotTexture,
    /// The HUD picker's preview of the highlighted HUD.
    hud_preview: LevelshotTexture,
    /// The model preview's display texture, once one exists.
    preview: Option<wgpu::BindGroup>,
    preview_sampler: wgpu::Sampler,
    /// Whether a frame past the vertex or run storage has been logged: shapes
    /// past it are dropped, so the log names the cause of a missing picture.
    overflow_logged: bool,
}

impl ShapeRenderer {
    /// Create the one process-lifetime pipeline and fixed vertex storage.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        format: wgpu::TextureFormat,
        depth_format: wgpu::TextureFormat,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("JKR retained UI shape shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("ui_shapes.wgsl").into()),
        });
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("JKR retained UI texture layout"),
            entries: &[
                crate::render_helpers::texture_layout_entry(0),
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("JKR retained UI shape pipeline layout"),
            bind_group_layouts: &[Some(&texture_layout)],
            immediate_size: 0,
        });
        let create_pipeline = |label, blend| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(label),
                layout: Some(&layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vertex_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers: &[Some(ShapeVertex::layout())],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fragment_main"),
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(blend),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: depth_format,
                    depth_write_enabled: Some(false),
                    depth_compare: Some(wgpu::CompareFunction::Always),
                    stencil: Default::default(),
                    bias: Default::default(),
                }),
                multisample: wgpu::MultisampleState::default(),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipeline = create_pipeline(
            "JKR retained UI shape pipeline",
            wgpu::BlendState::ALPHA_BLENDING,
        );
        let additive_pipeline =
            create_pipeline("SJK retained UI additive shape pipeline", ADDITIVE_BLENDING);
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("JKR retained UI shape vertices"),
            size: (MAX_SHAPE_VERTICES * std::mem::size_of::<ShapeVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let icons = IconAtlas::new(device, &texture_layout);
        let levelshot = LevelshotTexture::new(device, &texture_layout);
        let hud_preview = LevelshotTexture::new(device, &texture_layout);
        match image::load_from_memory(MENU_WORDMARK) {
            Ok(wordmark) => icons.upload_banner(queue, &wordmark.into_rgba8()),
            Err(error) => eprintln!("menu wordmark: {error}"),
        }
        let mut renderer = Self {
            pipeline,
            additive_pipeline,
            vertex_buffer,
            vertices: Vec::with_capacity(MAX_SHAPE_VERTICES),
            icons,
            runs: Vec::with_capacity(art::MAX_RUNS),
            texture_layout,
            art: ArtTextures::new(),
            emblem: EmblemTextures::new(),
            levelshot,
            hud_preview,
            preview: None,
            preview_sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some("SJK model preview sampler"),
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                ..Default::default()
            }),
            overflow_logged: false,
        };
        // Artwork decoded for an earlier world is uploaded with this one, on
        // the install worker rather than the frame thread.
        renderer.install_menu_art(device, queue);
        renderer
    }

    /// Upload the classic menu artwork once its decode has finished; does
    /// nothing before that or after it has been installed.
    pub(crate) fn install_menu_art(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        if self.art.installed() {
            return;
        }
        if let Some(decoded) = crate::menu::art::decoded() {
            self.art
                .install(device, queue, &self.texture_layout, decoded);
        }
    }

    /// Upload SJK's menu emblem once its decode has finished; does nothing
    /// before that or after it has been installed.
    pub(crate) fn install_emblem(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
    ) {
        if self.emblem.installed() {
            return;
        }
        if let Some(decoded) = crate::menu::emblem::decoded() {
            self.emblem
                .install(device, queue, &self.texture_layout, decoded);
        }
    }

    /// Classic menu art pieces this renderer can draw.
    pub(crate) fn menu_art(&self) -> crate::menu::art::ArtSet {
        self.art.ready()
    }

    /// Translate every active retained layer and upload one shared vertex batch.
    pub(crate) fn prepare_layers<'a>(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        draw_lists: impl IntoIterator<Item = &'a DrawList>,
        viewport: [f32; 2],
    ) {
        self.vertices.clear();
        self.runs.clear();
        self.runs.push(Run {
            start: 0,
            source: Source::Atlas,
        });
        self.art.begin_frame();
        let now = crate::menu::art::motion::seconds();
        let mut opacity = [1.0_f32; 8];
        let mut opacity_depth = 0_usize;
        let mut clips = [Rect::new(0.0, 0.0, viewport[0], viewport[1]); 8];
        let mut clip_depth = 0_usize;
        for command in draw_lists.into_iter().flat_map(DrawList::commands) {
            match *command {
                DrawCommand::SolidRect { rect, color }
                | DrawCommand::RoundedRect {
                    rect,
                    radius: 0.0,
                    color,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        color,
                        color,
                        0.0,
                        false,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::RoundedRect {
                    rect,
                    radius,
                    color,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        color,
                        color,
                        radius,
                        false,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::GradientRect {
                    rect,
                    radius,
                    gradient,
                } => {
                    self.push_rect(
                        clipped(rect, clips[clip_depth]),
                        gradient.start,
                        gradient.end,
                        radius,
                        gradient.vertical,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Border {
                    rect,
                    width,
                    color,
                    radius,
                } if radius > 0.0 => {
                    self.push_ring(
                        clipped(rect, clips[clip_depth]),
                        color,
                        radius,
                        width,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Border {
                    rect, width, color, ..
                } => {
                    for edge in border_rects(rect, width) {
                        self.push_rect(
                            clipped(edge, clips[clip_depth]),
                            color,
                            color,
                            0.0,
                            false,
                            opacity[opacity_depth],
                            viewport,
                        );
                    }
                }
                DrawCommand::PushClip(rect) if clip_depth + 1 < clips.len() => {
                    clip_depth += 1;
                    clips[clip_depth] = clipped(clips[clip_depth - 1], rect);
                }
                DrawCommand::PopClip => clip_depth = clip_depth.saturating_sub(1),
                DrawCommand::PushOpacity(value) if opacity_depth + 1 < opacity.len() => {
                    opacity_depth += 1;
                    opacity[opacity_depth] = opacity[opacity_depth - 1] * value.clamp(0.0, 1.0);
                }
                DrawCommand::PopOpacity => opacity_depth = opacity_depth.saturating_sub(1),
                DrawCommand::TexturedQuad {
                    rect,
                    texture,
                    color,
                } => {
                    let Some(source) = self.texture_source(queue, texture, now) else {
                        continue;
                    };
                    let uv = match source {
                        Source::Art(_)
                        | Source::Emblem(_)
                        | Source::Levelshot
                        | Source::HudPreview
                        | Source::Preview => ([0.0, 0.0], [1.0, 1.0]),
                        Source::Atlas => icons::uv_range(texture),
                    };
                    icons::push_quad(
                        &mut self.vertices,
                        clipped(rect, clips[clip_depth]),
                        uv,
                        color,
                        opacity[opacity_depth],
                        viewport,
                        MAX_SHAPE_VERTICES,
                    );
                }
                DrawCommand::TexturedQuadUv {
                    rect,
                    texture,
                    color,
                    uv,
                } => {
                    let Some(source) = self.texture_source(queue, texture, now) else {
                        continue;
                    };
                    // Explicit coordinates are mapped into an atlas cell;
                    // the quad is not clipped, as clipping would need its
                    // coordinates cut to match.
                    let uv = match source {
                        Source::Art(_)
                        | Source::Emblem(_)
                        | Source::Levelshot
                        | Source::HudPreview
                        | Source::Preview => uv,
                        Source::Atlas => {
                            let (low, high) = icons::uv_range(texture);
                            uv.map(|[s, t]| {
                                [
                                    low[0] + s * (high[0] - low[0]),
                                    low[1] + t * (high[1] - low[1]),
                                ]
                            })
                        }
                    };
                    icons::push_quad_corners(
                        &mut self.vertices,
                        rect,
                        uv,
                        color,
                        opacity[opacity_depth],
                        viewport,
                        MAX_SHAPE_VERTICES,
                    );
                }
                DrawCommand::Arc {
                    center,
                    radius,
                    width,
                    start,
                    sweep,
                    color,
                } => {
                    self.push_arc(
                        center,
                        radius,
                        width,
                        start,
                        sweep,
                        color,
                        opacity[opacity_depth],
                        viewport,
                    );
                }
                DrawCommand::Text { .. }
                | DrawCommand::PushClip(_)
                | DrawCommand::PushOpacity(_) => {}
            }
        }
        if self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            self.log_overflow("shape vertices");
        }
        if !self.vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    /// Log, once, that a frame ran out of `storage` and dropped shapes.
    fn log_overflow(&mut self, storage: &str) {
        if !std::mem::replace(&mut self.overflow_logged, true) {
            crate::log::progress(format_args!(
                "UI shapes dropped: the frame ran out of {storage}                  (limits {MAX_SHAPE_VERTICES} vertices, {} texture switches)",
                art::MAX_RUNS
            ));
        }
    }

    /// The source a textured quad naming `texture` samples, with its run
    /// begun and an animated art piece stepped to `now`; `None` when it
    /// draws nothing (art that is not loaded, whose screen falls back to
    /// vector shapes, or a full run list).
    fn texture_source(
        &mut self,
        queue: &crate::frame_queue::FrameQueue,
        texture: sjk_ui::TextureId,
        now: f64,
    ) -> Option<Source> {
        let source = match crate::menu::art::ArtPiece::from_texture(texture) {
            Some(piece) if self.art.ready().has(piece) => {
                self.art.animate(queue, piece, now);
                Source::Art(piece)
            }
            Some(_) => return None,
            None if texture == LEVELSHOT_TEXTURE => Source::Levelshot,
            None if texture == HUD_PREVIEW_TEXTURE => Source::HudPreview,
            None if texture == PREVIEW_TEXTURE => match self.preview {
                Some(_) => Source::Preview,
                None => return None,
            },
            None => match crate::menu::emblem::EmblemLayer::from_texture(texture) {
                Some(layer) if self.emblem.group(layer).is_some() => Source::Emblem(layer),
                Some(_) => return None,
                None => Source::Atlas,
            },
        };
        if art::switch(&mut self.runs, self.vertices.len(), source) {
            Some(source)
        } else {
            self.log_overflow("texture switches");
            None
        }
    }

    /// Draw every retained shape in one pipeline/buffer submission.
    pub(crate) fn draw<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        if self.vertices.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        let mut additive = false;
        let total = self.vertices.len() as u32;
        for (index, run) in self.runs.iter().enumerate() {
            let end = self.runs.get(index + 1).map_or(total, |next| next.start);
            if end <= run.start {
                continue;
            }
            if run.source.additive() != additive {
                additive = run.source.additive();
                pass.set_pipeline(if additive {
                    &self.additive_pipeline
                } else {
                    &self.pipeline
                });
            }
            let group = match run.source {
                Source::Art(piece) => self.art.group(piece),
                Source::Emblem(layer) => self.emblem.group(layer),
                Source::Levelshot => Some(self.levelshot.bind_group()),
                Source::HudPreview => Some(self.hud_preview.bind_group()),
                Source::Preview => self.preview.as_ref(),
                Source::Atlas => None,
            };
            pass.set_bind_group(0, group.unwrap_or(self.icons.bind_group()), &[]);
            pass.draw(run.start..end, 0..1);
        }
    }

    fn push_rect(
        &mut self,
        rect: Rect,
        start: Color,
        end: Color,
        radius: f32,
        vertical: bool,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        self.push_shape(
            rect,
            start,
            end,
            radius,
            f32::from(vertical),
            opacity,
            viewport,
        );
    }

    /// Rounded outline `width` pixels thick, drawn as one quad the shader
    /// hollows out (negative gradient parameter = ring width).
    fn push_ring(
        &mut self,
        rect: Rect,
        color: Color,
        radius: f32,
        width: f32,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        self.push_shape(
            rect,
            color,
            color,
            radius,
            -width.max(0.5),
            opacity,
            viewport,
        );
    }

    /// A round-capped arc stroke as one quad around its circle; the shader cuts out
    /// the stroke from a signed distance (mode 3, geometry in `end_color`).
    #[allow(clippy::too_many_arguments)]
    fn push_arc(
        &mut self,
        center: [f32; 2],
        radius: f32,
        width: f32,
        start: f32,
        sweep: f32,
        color: Color,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        // One pixel of margin leaves room for the anti-aliased edge.
        let extent = radius + width * 0.5 + 1.0;
        let rect = Rect::new(
            center[0] - extent,
            center[1] - extent,
            extent * 2.0,
            extent * 2.0,
        );
        if radius <= 0.0 || width <= 0.0 || self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            return;
        }
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let tint = [color.r, color.g, color.b, color.a * opacity];
        let points = [
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.y], [1.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [rect.width, rect.height],
                start_color: tint,
                end_color: [radius, width, start, sweep],
                parameters: [0.0, ARC_MODE],
                uv: local,
            }));
    }

    fn push_shape(
        &mut self,
        rect: Rect,
        start: Color,
        end: Color,
        radius: f32,
        mode: f32,
        opacity: f32,
        viewport: [f32; 2],
    ) {
        if rect.width <= 0.0 || rect.height <= 0.0 || self.vertices.len() + 6 > MAX_SHAPE_VERTICES {
            return;
        }
        let position = |x: f32, y: f32| [x / viewport[0] * 2.0 - 1.0, 1.0 - y / viewport[1] * 2.0];
        let color = |value: Color| [value.r, value.g, value.b, value.a * opacity];
        let points = [
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.y], [1.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.y], [0.0, 0.0]),
            ([rect.right(), rect.bottom()], [1.0, 1.0]),
            ([rect.x, rect.bottom()], [0.0, 1.0]),
        ];
        self.vertices
            .extend(points.map(|(pixel, local)| ShapeVertex {
                position: position(pixel[0], pixel[1]),
                local,
                size: [rect.width, rect.height],
                start_color: color(start),
                end_color: color(end),
                parameters: [radius.min(rect.width.min(rect.height) * 0.5), mode],
                uv: local,
            }));
    }

    /// Upload one decoded [`ICON_SIZE`]-square RGBA icon into a stable atlas
    /// cell, sampled by `TexturedQuad` commands naming `texture`.
    pub(crate) fn upload_icon(
        &self,
        queue: &crate::frame_queue::FrameQueue,
        texture: sjk_ui::TextureId,
        rgba: &[u8],
    ) {
        self.icons.upload(queue, texture, rgba);
    }

    /// Sample `view` for `TexturedQuad` commands naming [`PREVIEW_TEXTURE`]
    /// (the model preview's display texture, after it is made or resized).
    pub(crate) fn set_preview(&mut self, device: &wgpu::Device, view: &wgpu::TextureView) {
        self.preview = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("SJK model preview UI binding"),
            layout: &self.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.preview_sampler),
                },
            ],
        }));
    }

    /// Replace the map preview sampled by `TexturedQuad` commands naming
    /// [`LEVELSHOT_TEXTURE`] with `image`, at its own resolution.
    pub(crate) fn upload_levelshot(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        self.levelshot
            .upload(device, queue, &self.texture_layout, image);
    }

    /// Replace the HUD picker's preview sampled by `TexturedQuad` commands
    /// naming [`HUD_PREVIEW_TEXTURE`] with `image`.
    pub(crate) fn upload_hud_preview(
        &mut self,
        device: &wgpu::Device,
        queue: &crate::frame_queue::FrameQueue,
        image: &crate::menu::levelshot::LevelshotImage,
    ) {
        self.hud_preview
            .upload(device, queue, &self.texture_layout, image);
    }
}

fn clipped(left: Rect, right: Rect) -> Rect {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    Rect::new(
        x,
        y,
        left.right().min(right.right()) - x,
        left.bottom().min(right.bottom()) - y,
    )
}

fn border_rects(rect: Rect, width: f32) -> [Rect; 4] {
    [
        Rect::new(rect.x, rect.y, rect.width, width),
        Rect::new(rect.x, rect.bottom() - width, rect.width, width),
        Rect::new(rect.x, rect.y, width, rect.height),
        Rect::new(rect.right() - width, rect.y, width, rect.height),
    ]
}

/// Append all text commands without allocating; HUD rectangles are consumed by
/// the existing fullscreen quad pass. `style` scales and tracks every command
/// ([`text::TextStyle::NEUTRAL`] draws the layout as authored).
pub(crate) fn append_text_commands<'a>(
    draw_list: &DrawList,
    resolve: impl Fn(TextId) -> &'a str,
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    viewport: [f32; 2],
    style: text::TextStyle,
) {
    append_text_commands_where(
        draw_list,
        resolve,
        |_, _| true,
        vertices,
        font,
        viewport,
        style,
    );
}

/// Append the text commands `keep` accepts (given each command's id and text),
/// so one draw list can be split between fonts. Opacity and clip scopes apply
/// to every pass alike, and `style` as in [`append_text_commands`].
pub(crate) fn append_text_commands_where<'a>(
    draw_list: &DrawList,
    resolve: impl Fn(TextId) -> &'a str,
    keep: impl Fn(TextId, &str) -> bool,
    vertices: &mut Vec<TextVertex>,
    font: &UiFont,
    viewport: [f32; 2],
    style: text::TextStyle,
) {
    let mut opacity = [1.0_f32; 8];
    let mut opacity_depth = 0_usize;
    let mut clips = [Rect::new(0.0, 0.0, viewport[0], viewport[1]); 8];
    let mut clip_depth = 0_usize;
    for command in draw_list.commands() {
        let DrawCommand::Text {
            rect,
            text: id,
            size,
            color,
            align,
            weight,
            letter_spacing,
            overflow,
        } = command
        else {
            match command {
                DrawCommand::PushOpacity(value) if opacity_depth + 1 < opacity.len() => {
                    opacity_depth += 1;
                    opacity[opacity_depth] = opacity[opacity_depth - 1] * value.clamp(0.0, 1.0);
                }
                DrawCommand::PopOpacity => opacity_depth = opacity_depth.saturating_sub(1),
                DrawCommand::PushClip(rect) if clip_depth + 1 < clips.len() => {
                    clip_depth += 1;
                    clips[clip_depth] = clipped(clips[clip_depth - 1], *rect);
                }
                DrawCommand::PopClip => clip_depth = clip_depth.saturating_sub(1),
                _ => {}
            }
            continue;
        };
        let value = resolve(*id);
        if !keep(*id, value) {
            continue;
        }
        let placement = style.place(*rect, *size, *letter_spacing);
        let rect = placement.bounds;
        let scale = placement.size / font.height.max(1.0);
        let face = match weight {
            FontWeight::Regular => text::TextFace::Regular,
            FontWeight::Semibold => text::TextFace::Semibold,
        };
        let measured =
            text::visible_text_width_style(font, value, scale, face, placement.letter_spacing);
        let width = if *overflow == sjk_ui::TextOverflow::Ellipsis {
            measured.min(rect.width)
        } else {
            measured
        };
        let x = match align {
            TextAlign::Start => rect.x,
            TextAlign::Center => rect.x + (rect.width - width) * 0.5,
            TextAlign::End => rect.right() - width,
        };
        text::append_bounded(
            vertices,
            font,
            value,
            [x, placement.y],
            rect,
            clipped(rect, clips[clip_depth]),
            scale,
            viewport,
            face,
            [color.r, color.g, color.b, color.a * opacity[opacity_depth]],
            placement.letter_spacing,
            *overflow,
        );
    }
}
