//! Optional load-time mip chains. Level zero uses the existing resize policy unchanged.
use image::{RgbaImage, imageops};
use std::{error::Error, sync::Arc};

#[path = "texture_mip_cache.rs"]
pub(crate) mod cache;

/// Number of complete mip levels through 1x1, including rectangular/NPOT images.
pub(crate) fn levels(width: u32, height: u32) -> u32 {
    32 - width.max(height).max(1).leading_zeros()
}

/// One layer's mip chain, level zero first: `image` fitted to `width` x `height`, then
/// halved down to one texel. Pure processor work, so a loader may build many at once.
pub(crate) fn chain(image: &RgbaImage, width: u32, height: u32) -> Vec<RgbaImage> {
    let mut chain = Vec::with_capacity(levels(width, height) as usize);
    chain.push(if image.width() == width && image.height() == height {
        image.clone()
    } else {
        imageops::resize(image, width, height, imageops::FilterType::Lanczos3)
    });
    for _ in 1..levels(width, height) {
        let above = chain.last().expect("level zero is there");
        chain.push(imageops::resize(
            above,
            (above.width() / 2).max(1),
            (above.height() / 2).max(1),
            imageops::FilterType::Triangle,
        ));
    }
    chain
}

/// The extent of the array `images` become layers of.
pub(crate) fn extent(images: &[Arc<RgbaImage>]) -> (u32, u32) {
    (
        images.iter().map(|v| v.width()).max().unwrap_or(1),
        images.iter().map(|v| v.height()).max().unwrap_or(1),
    )
}

/// What the worlds of one context share a map texture as ([`crate::shared_textures`]):
/// the view, and the images it was made from, held so their addresses (the key) cannot
/// name other pixels while it lives.
type Shared = (wgpu::TextureView, Arc<[Arc<RgbaImage>]>);
const SHARED: &str = "map texture";

/// The key of the `width` x `height` array `images` become. Decoded map images are
/// one `Arc` per mounted file and content for the process
/// ([`crate::decoded_image_cache`]), so two maps using the same file give one key.
fn shared_key(images: &[Arc<RgbaImage>], width: u32, height: u32) -> u64 {
    let mut parts = vec![width.to_le_bytes().to_vec(), height.to_le_bytes().to_vec()];
    parts.extend(
        images
            .iter()
            .map(|image| (Arc::as_ptr(image) as usize as u64).to_le_bytes().to_vec()),
    );
    crate::shared_textures::key(&parts.iter().map(Vec::as_slice).collect::<Vec<_>>())
}

/// The mipmapped array of `images` another world of this context holds, if any.
pub(crate) fn find_shared(images: &[Arc<RgbaImage>]) -> Option<wgpu::TextureView> {
    let (width, height) = extent(images);
    crate::shared_textures::find::<Shared>(SHARED, shared_key(images, width, height))
        .map(|(view, _)| view)
}

/// [`upload_chains`] of `images`, or another world's copy of the same array.
pub(crate) fn upload_shared(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    images: &[Arc<RgbaImage>],
    width: u32,
    height: u32,
    chains: impl FnOnce() -> Arc<Vec<Vec<RgbaImage>>>,
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let key = shared_key(images, width, height);
    if let Some((view, _)) = crate::shared_textures::find::<Shared>(SHARED, key) {
        return Ok(view);
    }
    let view = upload_chains(device, queue, width, height, &chains())?;
    let shared: Shared = (view.clone(), images.into());
    crate::shared_textures::share(SHARED, key, || shared);
    Ok(view)
}

/// Upload mipmapped stage frames once; no frame-path image generation or readback.
pub(crate) fn upload(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    images: &[Arc<RgbaImage>],
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let (width, height) = extent(images);
    upload_shared(device, queue, images, width, height, || {
        cache::prepare(images, width, height)
    })
}

/// Upload one [`chain`] per layer of a `width` x `height` array.
pub(crate) fn upload_chains(
    device: &wgpu::Device,
    queue: &crate::frame_queue::FrameQueue,
    width: u32,
    height: u32,
    chains: &[Vec<RgbaImage>],
) -> Result<wgpu::TextureView, Box<dyn Error>> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("SJK optional mipmapped material"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: chains.len().max(1).try_into()?,
        },
        mip_level_count: levels(width, height),
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    for (layer, chain) in chains.iter().enumerate() {
        for (level, pixels) in chain.iter().enumerate() {
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level.try_into()?,
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: 0,
                        z: layer.try_into()?,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                pixels.as_raw(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(pixels.width() * 4),
                    rows_per_image: Some(pixels.height()),
                },
                wgpu::Extent3d {
                    width: pixels.width(),
                    height: pixels.height(),
                    depth_or_array_layers: 1,
                },
            );
        }
    }
    Ok(texture.create_view(&wgpu::TextureViewDescriptor {
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    }))
}
