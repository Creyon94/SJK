//! Silent, bounded AVC/H.264 MP4 clips, decoded on the media worker, never the frame
//! thread. Audio tracks are ignored. Fragmented MP4, HEVC and WebM are unsupported.

use super::decode::{Frame, Gif, MAX_CANVAS, MAX_DECODED, MAX_FRAMES, fitted};
use openh264::{
    decoder::{Decoder, DecoderConfig, Flush},
    formats::YUVSource,
};
use std::io::Cursor;

/// MP4 tables and nested boxes are checked before the third-party parser allocates.
/// Unknown metadata is hidden in-place as `free`, keeping media offsets unchanged.
pub(crate) fn decode(bytes: &[u8]) -> Option<Gif> {
    if bytes.len() as u64 > super::fetch::MAX_BYTES || bytes.get(4..8) != Some(b"ftyp") {
        return None;
    }
    let mut checked = bytes.to_vec();
    boxes(&mut checked, 0, &mut 0)?;
    std::panic::catch_unwind(|| decode_checked(&checked))
        .ok()
        .flatten()
}
fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        bytes.get(at..at.checked_add(4)?)?.try_into().ok()?,
    ))
}
fn boxes(bytes: &mut [u8], depth: usize, count: &mut usize) -> Option<()> {
    if depth > 8 {
        return None;
    }
    let mut offset = 0;
    while offset < bytes.len() {
        *count += 1;
        if *count > 4096 {
            return None;
        }
        let size = u32_at(bytes, offset)? as usize;
        if size < 8 {
            return None;
        } // no extended/zero-sized boxes in this subset
        let atom = bytes.get_mut(offset..offset.checked_add(size)?)?;
        let kind: [u8; 4] = atom[4..8].try_into().ok()?;
        let body = &mut atom[8..];
        match &kind {
            b"moov" | b"trak" | b"mdia" | b"minf" | b"stbl" | b"dinf" => {
                boxes(body, depth + 1, count)?
            }
            b"moof" | b"mvex" => return None,
            b"stsd" => {
                if u32_at(body, 4)? > 16 {
                    return None;
                }
                boxes(body.get_mut(8..)?, depth + 1, count)?;
            }
            b"dref" => {
                if u32_at(body, 4)? != 1 {
                    return None;
                }
                boxes(body.get_mut(8..)?, depth + 1, count)?;
            }
            b"url " => {
                if body != [0, 0, 0, 1] {
                    return None;
                }
            }
            b"avc1" => boxes(body.get_mut(78..)?, depth + 1, count)?,
            b"stts" | b"ctts" | b"stsc" | b"stco" | b"co64" | b"stss" => {
                let entries = u32_at(body, 4)? as usize;
                let stride = match &kind {
                    b"stsc" => 12,
                    b"stts" | b"ctts" | b"co64" => 8,
                    _ => 4,
                };
                if entries > 4096 || entries.checked_mul(stride)?.checked_add(8)? > body.len() {
                    return None;
                }
                if matches!(&kind, b"stts" | b"ctts") {
                    let mut samples = 0u32;
                    for row in body[8..8 + entries * stride].chunks_exact(stride) {
                        samples = samples.checked_add(u32_at(row, 0)?)?;
                    }
                    if samples > 4096 {
                        return None;
                    }
                }
            }
            b"stsz" => {
                let size = u32_at(body, 4)?;
                let samples = u32_at(body, 8)? as usize;
                if samples > 4096 || size as u64 > super::fetch::MAX_BYTES {
                    return None;
                }
                if size == 0 {
                    if samples.checked_mul(4)?.checked_add(12)? > body.len() {
                        return None;
                    }
                    for row in body[12..12 + samples * 4].as_chunks::<4>().0 {
                        if u32_at(row, 0)? as u64 > super::fetch::MAX_BYTES {
                            return None;
                        }
                    }
                }
            }
            b"ftyp" => {
                if body.len() < 8 || body.len() % 4 != 0 {
                    return None;
                }
            }
            b"hdlr" => {
                if body.len() < 24 {
                    return None;
                }
            }
            b"mvhd" | b"tkhd" | b"mdhd" => {
                if body.is_empty() || body[0] > 1 {
                    return None;
                }
                let minimum = match &kind {
                    b"mvhd" => {
                        if body[0] == 1 {
                            112
                        } else {
                            100
                        }
                    }
                    b"tkhd" => {
                        if body[0] == 1 {
                            96
                        } else {
                            84
                        }
                    }
                    _ => {
                        if body[0] == 1 {
                            36
                        } else {
                            24
                        }
                    }
                };
                if body.len() < minimum {
                    return None;
                }
            }
            b"avcC" => {
                if body.len() > 65536 {
                    return None;
                }
            }
            b"mdat" | b"free" | b"vmhd" => {}
            _ => atom[4..8].copy_from_slice(b"free"),
        }
        offset += size;
    }
    Some(())
}
fn decode_checked(bytes: &[u8]) -> Option<Gif> {
    let mut mp4 = mp4::Mp4Reader::read_header(Cursor::new(bytes), bytes.len() as u64).ok()?;
    let track = mp4
        .tracks()
        .values()
        .find(|track| track.trak.mdia.minf.stbl.stsd.avc1.is_some())?;
    let id = track.track_id();
    let scale = track.timescale();
    let samples = track.sample_count();
    if scale == 0
        || samples == 0
        || samples > 4096
        || track.width() == 0
        || track.height() == 0
        || u32::from(track.width().max(track.height())) > MAX_CANVAS
    {
        return None;
    }
    let avcc = &track.trak.mdia.minf.stbl.stsd.avc1.as_ref()?.avcc;
    let prefix = usize::from(avcc.length_size_minus_one & 3) + 1;
    let mut decoder = Decoder::with_api_config(
        openh264::OpenH264API::from_source(),
        DecoderConfig::new().flush_after_decode(Flush::NoFlush),
    )
    .ok()?;
    let mut packet = Vec::new();
    for parameter in avcc
        .sequence_parameter_sets
        .iter()
        .chain(&avcc.picture_parameter_sets)
    {
        append_nal(&mut packet, &parameter.bytes)?;
    }
    decoder.decode(&packet).ok()?;
    let mut frames = Vec::new();
    let mut size = None;
    let mut last_delay = 33;
    let mut timing = Vec::new();
    let started = std::time::Instant::now();
    for index in 1..=samples.min(MAX_FRAMES as u32 + 1) {
        if started.elapsed().as_secs() >= 5 {
            return None;
        }
        let sample = mp4.read_sample(id, index).ok()??;
        timing.push((
            i128::from(sample.start_time) + i128::from(sample.rendering_offset),
            sample.duration,
        ));
        packet.clear();
        let mut rest = sample.bytes.as_ref();
        while !rest.is_empty() {
            let head = rest.get(..prefix)?;
            let length = head
                .iter()
                .fold(0usize, |value, byte| (value << 8) | usize::from(*byte));
            rest = rest.get(prefix..)?;
            let nal = rest.get(..length)?;
            append_nal(&mut packet, nal)?;
            rest = rest.get(length..)?;
        }
        last_delay =
            ((u64::from(sample.duration) * 1000) / u64::from(scale)).clamp(10, 1000) as u32;
        if let Some(yuv) = decoder.decode(&packet).ok()? {
            push_frame(&yuv, last_delay, &mut size, &mut frames)?;
        }
        if frames.len() >= MAX_FRAMES
            || size.is_some_and(|[w, h]| {
                (frames.len() + 1) * w as usize * h as usize * 4 > MAX_DECODED
            })
        {
            break;
        }
    }
    // Flush delayed pictures (e.g. B frames) in presentation order.
    for yuv in decoder.flush_remaining().ok()? {
        if frames.len() >= MAX_FRAMES
            || size.is_some_and(|[w, h]| {
                (frames.len() + 1) * w as usize * h as usize * 4 > MAX_DECODED
            })
        {
            break;
        }
        push_frame(&yuv, last_delay, &mut size, &mut frames)?;
    }
    let size = size?;
    timing.sort_by_key(|(pts, _)| *pts);
    for (frame, (_, duration)) in frames.iter_mut().zip(timing) {
        frame.delay_ms = ((u64::from(duration) * 1000) / u64::from(scale)).clamp(10, 1000) as u32;
    }
    let length_ms = frames.iter().map(|frame| u64::from(frame.delay_ms)).sum();
    (!frames.is_empty()).then_some(Gif {
        size,
        frames,
        length_ms,
    })
}
fn push_frame(
    yuv: &openh264::decoder::DecodedYUV<'_>,
    delay_ms: u32,
    size: &mut Option<[u32; 2]>,
    frames: &mut Vec<Frame>,
) -> Option<()> {
    let (width, height) = yuv.dimensions();
    if width == 0 || height == 0 || width.max(height) > MAX_CANVAS as usize {
        return None;
    }
    let fitted = fitted([width as u32, height as u32]);
    if size.is_some_and(|size| size != fitted) {
        return None;
    }
    *size = Some(fitted);
    let mut rgba = vec![0; width.checked_mul(height)?.checked_mul(4)?];
    yuv.write_rgba8(&mut rgba);
    let image = image::RgbaImage::from_raw(width as u32, height as u32, rgba)?;
    let image = if [width as u32, height as u32] == fitted {
        image
    } else {
        image::imageops::resize(
            &image,
            fitted[0],
            fitted[1],
            image::imageops::FilterType::Triangle,
        )
    };
    frames.push(Frame {
        rgba: image.into_raw().into_boxed_slice(),
        delay_ms,
    });
    Some(())
}
/// Validate all SPSs before native decode, including SPS changes inside samples.
fn append_nal(out: &mut Vec<u8>, nal: &[u8]) -> Option<()> {
    if nal.first()? & 128 != 0 {
        return None;
    }
    let kind = nal.first()? & 31;
    if kind == 7 {
        let rbsp = h264_reader::rbsp::decode_nal(nal).ok()?;
        let sps = h264_reader::nal::sps::SeqParameterSet::from_bits(
            h264_reader::rbsp::BitReader::new(rbsp.as_ref()),
        )
        .ok()?;
        if sps.chroma_info.bit_depth_luma_minus8 != 0
            || sps.chroma_info.bit_depth_chroma_minus8 != 0
        {
            return None;
        }
        let row_height = if matches!(
            sps.frame_mbs_flags,
            h264_reader::nal::sps::FrameMbsFlags::Frames
        ) {
            16
        } else {
            32
        };
        if sps.chroma_info.chroma_format != h264_reader::nal::sps::ChromaFormat::YUV420 {
            return None;
        }
        if sps.pic_width_in_mbs_minus1 >= MAX_CANVAS / 16
            || sps.pic_height_in_map_units_minus1 >= MAX_CANVAS / row_height
            || sps.max_num_ref_frames > 8
        {
            return None;
        }
        let (w, h) = sps.pixel_dimensions().ok()?;
        if w == 0 || h == 0 || w.max(h) > MAX_CANVAS {
            return None;
        }
    }
    // Extension parameter sets and scalable/multiview streams are unsupported.
    if !matches!(kind, 1..=9 | 12) {
        return None;
    }
    if out.len().checked_add(nal.len())?.checked_add(4)? > super::fetch::MAX_BYTES as usize {
        return None;
    }
    out.extend_from_slice(&[0, 0, 0, 1]);
    out.extend_from_slice(nal);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_mp4_boxes_and_huge_tables_are_refused_before_parsing() {
        for bytes in [
            b"\0\0\0\x01ftyp".as_slice(),
            b"\0\0\0\x04ftyp",
            b"\xff\xff\xff\xffftyp",
            b"\0\0\0\x10stco\0\0\0\0\xff\xff\xff\xff",
        ] {
            assert!(boxes(&mut bytes.to_vec(), 0, &mut 0).is_none());
        }
        assert!(decode(b"not a video").is_none());
    }
}

#[cfg(test)]
mod clip_tests {
    use super::*;
    /// An original synthetic clip, encoded in memory; no downloaded fixture/assets.
    pub(crate) fn clip() -> Vec<u8> {
        use openh264::{
            encoder::Encoder,
            formats::{RgbSliceU8, YUVBuffer},
        };
        let mut encoder = Encoder::new().unwrap();
        let mut samples = Vec::new();
        let mut sps = Vec::new();
        let mut pps = Vec::new();
        for color in [[220, 30, 20], [20, 210, 30], [20, 30, 220]] {
            let rgb = color.repeat(32 * 32);
            let yuv = YUVBuffer::from_rgb_source(RgbSliceU8::new(&rgb, (32, 32)));
            let encoded = encoder.encode(&yuv).unwrap().to_vec();
            let mut sample = Vec::new();
            for nal in openh264::nal_units(&encoded) {
                let nal = nal
                    .strip_prefix(&[0, 0, 0, 1])
                    .or_else(|| nal.strip_prefix(&[0, 0, 1]))
                    .unwrap_or(nal);
                match nal[0] & 31 {
                    7 => sps = nal.to_vec(),
                    8 => pps = nal.to_vec(),
                    _ => {
                        sample.extend_from_slice(&(nal.len() as u32).to_be_bytes());
                        sample.extend_from_slice(nal);
                    }
                }
            }
            samples.push(sample);
        }
        let config = mp4::Mp4Config {
            major_brand: "isom".parse().unwrap(),
            minor_version: 0,
            compatible_brands: vec!["isom".parse().unwrap()],
            timescale: 1000,
        };
        let mut writer = mp4::Mp4Writer::write_start(Cursor::new(Vec::new()), &config).unwrap();
        let mut track: mp4::TrackConfig = mp4::AvcConfig {
            width: 32,
            height: 32,
            seq_param_set: sps,
            pic_param_set: pps,
        }
        .into();
        track.timescale = 1000;
        writer.add_track(&track).unwrap();
        for (index, bytes) in samples.into_iter().enumerate() {
            writer
                .write_sample(
                    1,
                    &mp4::Mp4Sample {
                        start_time: index as u64 * 100,
                        duration: 100,
                        rendering_offset: 0,
                        is_sync: index == 0,
                        bytes: bytes.into(),
                    },
                )
                .unwrap();
        }
        writer.write_end().unwrap();
        writer.into_writer().into_inner()
    }
    #[test]
    fn silent_mp4_decodes_original_generated_frames_and_timing() {
        let bytes = clip();
        let mut checked = bytes.clone();
        assert!(
            boxes(&mut checked, 0, &mut 0).is_some(),
            "bounded MP4 structure"
        );
        mp4::Mp4Reader::read_header(Cursor::new(&checked), checked.len() as u64)
            .expect("sanitized MP4 parses");
        let gif = decode(&bytes).expect("synthetic AVC MP4 decodes");
        assert_eq!(gif.size, [32, 32]);
        assert_eq!(gif.frames.len(), 3);
        assert_eq!(gif.length_ms, 300);
        for (frame, dominant) in gif.frames.iter().zip(0..3) {
            let pixel = &frame.rgba[..4];
            assert!(pixel[dominant] > 150, "{pixel:?}");
            assert_eq!(pixel[3], 255);
        }
        assert_eq!(gif.frame_at(100), 1);
        assert_eq!(gif.frame_at(300), 0);
    }
}
