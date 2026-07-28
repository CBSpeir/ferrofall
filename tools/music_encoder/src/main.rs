use std::env;
use std::fs;
use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;

use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = env::args_os().skip(1);
    let input = arguments.next().ok_or("missing input WAV path")?;
    let output = arguments.next().ok_or("missing output Ogg path")?;
    if arguments.next().is_some() {
        return Err("expected exactly an input and output path".into());
    }

    let (sample_rate, samples) = read_mono_pcm16_wav(Path::new(&input))?;
    let mut builder = VorbisEncoderBuilder::new_with_serial(
        NonZeroU32::new(sample_rate).ok_or("sample rate must be positive")?,
        NonZeroU8::new(1).expect("one is nonzero"),
        Vec::new(),
        0x4f58_4944,
    );
    builder.bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: 0.35,
    });
    let mut encoder = builder.build()?;
    for chunk in samples.chunks(4096) {
        encoder.encode_audio_block([chunk])?;
    }
    fs::write(output, encoder.finish()?)?;
    Ok(())
}

fn read_mono_pcm16_wav(path: &Path) -> Result<(u32, Vec<f32>), Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    if bytes.len() < 44 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("input is not a RIFF WAVE file".into());
    }

    let mut sample_rate = None;
    let mut data = None;
    let mut offset = 12;
    while offset + 8 <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let length = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
        let start = offset + 8;
        let end = start.checked_add(length).ok_or("invalid WAV chunk length")?;
        if end > bytes.len() {
            return Err("WAV chunk extends past end of file".into());
        }
        if id == b"fmt " {
            if length < 16 {
                return Err("WAV format chunk is too short".into());
            }
            let format = u16::from_le_bytes(bytes[start..start + 2].try_into()?);
            let channels = u16::from_le_bytes(bytes[start + 2..start + 4].try_into()?);
            let rate = u32::from_le_bytes(bytes[start + 4..start + 8].try_into()?);
            let bits = u16::from_le_bytes(bytes[start + 14..start + 16].try_into()?);
            if format != 1 || channels != 1 || bits != 16 {
                return Err("expected mono, 16-bit PCM WAV input".into());
            }
            sample_rate = Some(rate);
        } else if id == b"data" {
            data = Some(&bytes[start..end]);
        }
        offset = end + (length % 2);
    }

    let sample_rate = sample_rate.ok_or("WAV format chunk is missing")?;
    let data = data.ok_or("WAV data chunk is missing")?;
    if data.len() % 2 != 0 {
        return Err("WAV data has a partial sample".into());
    }
    let samples = data
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]) as f32 / 32_768.0)
        .collect();
    Ok((sample_rate, samples))
}
