//! The GPU copy of the wide-glyph atlas ([`super::wide`]) and the text bind groups
//! that carry it.
//!
//! Every text bind group holds its font's own atlas at binding 0, the sampler at 1 and
//! the wide atlas at 2; `text.wgsl` samples the wide atlas for vertices whose `u` is
//! [`super::wide::U_OFFSET`] or more.

use super::wide::{self, ATLAS_HEIGHT, ATLAS_WIDTH, Cell};
use crate::frame_queue::FrameQueue;

/// The wide atlas on the GPU: R8 coverage with the code-page atlases' mip levels.
pub(crate) struct WideTexture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    /// Reused list of cells to copy this frame.
    cells: Vec<Cell>,
}

impl WideTexture {
    /// An empty atlas. The CPU atlas starts over with it, so every wide glyph on
    /// screen is rasterized into this texture over the next frames.
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("SJK wide glyph atlas"),
            size: wgpu::Extent3d {
                width: ATLAS_WIDTH,
                height: ATLAS_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: super::ATLAS_MIP_LEVELS,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        wide::start_over();
        Self {
            texture,
            view,
            cells: Vec::new(),
        }
    }

    /// The atlas, for binding 2 of a text bind group.
    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    /// Rasterize the glyphs this frame asked for (within [`wide::FRAME_BUDGET`]) and
    /// copy every new cell, with its mip levels, into the texture ahead of the next
    /// submission. Nothing is copied or allocated on a frame that asked for no new glyph.
    pub(crate) fn update(&mut self, queue: &FrameQueue) {
        wide::rasterize_pending(wide::FRAME_BUDGET);
        wide::take_dirty(&mut self.cells);
        if self.cells.is_empty() {
            return;
        }
        wide::mirror(|pixels| {
            for &cell in &self.cells {
                for level in 0..super::ATLAS_MIP_LEVELS {
                    let data = wide::cell_level(pixels, cell, level);
                    let (width, height) = (cell.width >> level, cell.height >> level);
                    queue.write_texture(
                        wgpu::TexelCopyTextureInfo {
                            texture: &self.texture,
                            mip_level: level,
                            origin: wgpu::Origin3d {
                                x: cell.x >> level,
                                y: cell.y >> level,
                                z: 0,
                            },
                            aspect: wgpu::TextureAspect::All,
                        },
                        &data,
                        wgpu::TexelCopyBufferLayout {
                            offset: 0,
                            bytes_per_row: Some(width),
                            rows_per_image: Some(height),
                        },
                        wgpu::Extent3d {
                            width,
                            height,
                            depth_or_array_layers: 1,
                        },
                    );
                }
            }
        });
        self.cells.clear();
    }
}

/// A text bind group: `atlas` (the font's own), the text sampler and the wide atlas.
pub(crate) fn bind_group(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::BindGroupLayout,
    atlas: &wgpu::TextureView,
    sampler: &wgpu::Sampler,
    wide: &wgpu::TextureView,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some(label),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(atlas),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(wide),
            },
        ],
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_text_program_validates_and_shares_the_wide_offset() {
        let source = include_str!("../text.wgsl");
        crate::wgsl_source::validate(source);
        let declared = format!("const WIDE_U_OFFSET: f32 = {:.1};", super::wide::U_OFFSET);
        assert!(source.contains(&declared), "{declared}");
    }

    /// The game chat with Russian and Japanese lines as they arrive: Windows-1251 from
    /// an EternalJK player, UTF-8 from SJK players, and a draft typed in Russian and
    /// Japanese, in Inter and in the game fonts (SJK Chat), at 1080p and at 4K.
    #[test]
    #[ignore = "renders with the GPU and the installed game data named by JKA_GAME_DATA"]
    fn chat_in_russian_and_japanese() {
        for (size, game_font, suffix) in [
            ([1920, 1080], "0", "1080p"),
            ([3840, 2160], "0", "4k"),
            ([1920, 1080], "1", "1080p-game-font"),
        ] {
            crate::world_shot::on_big_stack(move || {
                let Some((mut gpu, _profile)) = crate::world_shot::open(
                    "maps/mp/duel6.bsp",
                    size,
                    None,
                    &[("cg_chatbox", "60000"), (crate::game_font::CVAR, game_font)],
                ) else {
                    return;
                };
                gpu.chat.for_shot = true;
                let lines: [&[u8]; 5] = [
                    // An EternalJK player with `se_language russian`: Windows-1251.
                    b"^1Vasya^7: \xef\xf0\xe8\xe2\xe5\xf2, \xea\xf2\xee \xed\xe0 \xe4\xf3\xfd\xeb\xfc?",
                    "^5Sol^7: Да, иду! Ёлки-палки, ПРИВЕТ всем".as_bytes(),
                    "^3Kenji^7: こんにちは、日本語のチャットです。漢字もカタカナも OK".as_bytes(),
                    "^2Mixed^7: gg привет 日本 café ½ €5".as_bytes(),
                    "^6Emoji^7: no font has it 😀".as_bytes(),
                ];
                for line in lines {
                    gpu.chat.receive(
                        sjk_client::ServerEventKind::Chat,
                        sjk_client::decode_legacy(line).into_owned(),
                        None,
                        std::time::Instant::now(),
                    );
                }
                gpu.chat.draft_for_shot("пишу по-русски и 日本語");
                let path = crate::world_shot::shoot(&mut gpu, 8, &format!("chat-unicode-{suffix}"));
                println!("{}", path.display());
            });
        }
    }
}
