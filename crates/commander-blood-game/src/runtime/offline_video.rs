//! Timestamped lossless RGB frames using the existing verified VP9 configuration.

use std::fs::File;
use std::num::NonZero;
use std::path::Path;

use anyhow::{Context, Result, ensure};
use vpx_rs::{
    Encoder, EncoderFrameFlags, EncodingDeadline, ImageFormat, Packet, Timebase, YUVImageData,
};
use webm::mux::{Segment, VideoTrack};

use crate::video_import::{lossless_rgb_encoder, lossless_rgb_segment};

pub(super) struct OfflineVideoWriter {
    encoder: Encoder<u8>,
    segment: Option<Segment<File>>,
    track: VideoTrack,
    width: usize,
    height: usize,
    planar: Vec<u8>,
    next_ns: u64,
}

impl OfflineVideoWriter {
    pub fn new(path: &Path, width: u32, height: u32) -> Result<Self> {
        let encoder = lossless_rgb_encoder(
            width,
            height,
            Timebase {
                num: NonZero::new(1).unwrap(),
                den: NonZero::new(1000).unwrap(),
            },
        )?;
        let (segment, track) = lossless_rgb_segment(path, width, height)?;
        Ok(Self {
            encoder,
            segment: Some(segment),
            track,
            width: width as usize,
            height: height as usize,
            planar: vec![0; width as usize * height as usize * 3],
            next_ns: 0,
        })
    }

    pub fn write(&mut self, start_ns: u64, duration_ns: u64, rgba: &[u8]) -> Result<()> {
        ensure!(start_ns == self.next_ns, "noncontiguous RGB video interval");
        ensure!(
            start_ns % 1_000_000 == 0 && duration_ns % 1_000_000 == 0 && duration_ns != 0,
            "native frame wait is not representable in Matroska milliseconds"
        );
        let pixels = self.width * self.height;
        ensure!(rgba.len() == pixels * 4, "wrong RGB frame size");
        let (green, blue_red) = self.planar.split_at_mut(pixels);
        let (blue, red) = blue_red.split_at_mut(pixels);
        for (i, pixel) in rgba.chunks_exact(4).enumerate() {
            ensure!(
                pixel[3] == 255,
                "final GPU composition contains nonopaque pixels"
            );
            red[i] = pixel[0];
            green[i] = pixel[1];
            blue[i] = pixel[2];
        }
        let image =
            YUVImageData::from_raw_data(ImageFormat::I444, self.width, self.height, &self.planar)
                .context("wrapping final RGB frame")?;
        let packets = self.encoder.encode(
            i64::try_from(start_ns / 1_000_000)?,
            duration_ns / 1_000_000,
            image,
            EncodingDeadline::GoodQuality,
            EncoderFrameFlags::empty(),
        )?;
        let mut emitted = 0;
        for packet in packets {
            if let Packet::CompressedFrame(frame) = packet {
                ensure!(
                    u64::try_from(frame.pts)? * 1_000_000 == start_ns,
                    "RGB encoder shifted timestamp"
                );
                self.segment
                    .as_mut()
                    .context("video already finalized")?
                    .add_frame(self.track, &frame.data, start_ns, frame.flags.is_key)?;
                emitted += 1;
            }
        }
        ensure!(
            emitted == 1,
            "zero-lag RGB encoder did not emit exactly one frame"
        );
        self.next_ns = start_ns
            .checked_add(duration_ns)
            .context("RGB timeline overflow")?;
        Ok(())
    }

    pub fn finish(&mut self) -> Result<()> {
        self.segment
            .take()
            .context("video already finalized")?
            .finalize(Some(self.next_ns))
            .map_err(|_| anyhow::anyhow!("finalizing timestamped RGB video"))?;
        Ok(())
    }
}
