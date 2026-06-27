use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use parking_lot::Mutex;
use std::sync::Arc;

pub type SharedBuffer = Arc<Mutex<Vec<f32>>>;

pub fn list_input_devices() -> Result<Vec<String>> {
    let host = cpal::default_host();
    let mut out = Vec::new();
    for d in host.input_devices()? {
        out.push(d.name().unwrap_or_default());
    }
    Ok(out)
}

pub fn open_input_stream(_device_id: Option<&str>, buf: SharedBuffer) -> Result<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_input_device().ok_or_else(|| anyhow!("no input device"))?;

    let config = device.default_input_config()?;
    let sample_format = config.sample_format();
    let sample_rate = config.sample_rate().0;
    let channels = config.channels() as usize;

    let err_fn = |err| tracing::error!("audio stream error: {err}");
    let buf_clone = buf.clone();

    let stream_config = StreamConfig {
        channels: config.channels(),
        sample_rate: config.sample_rate(),
        buffer_size: cpal::BufferSize::Default,
    };

    let stream = match sample_format {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _: &cpal::InputCallbackInfo| {
                let mono: Vec<f32> = data.chunks(channels).map(|c| c[0]).collect();
                let mut b = buf_clone.lock();
                b.extend(mono);
            },
            err_fn,
            None,
        )?,
        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _: &cpal::InputCallbackInfo| {
                let mono: Vec<f32> = data.chunks(channels).map(|c| c[0] as f32 / 32768.0).collect();
                let mut b = buf_clone.lock();
                b.extend(mono);
            },
            err_fn,
            None,
        )?,
        _ => return Err(anyhow!("unsupported sample format")),
    };
    stream.play()?;
    tracing::info!("audio stream started at {} Hz, {} ch", sample_rate, channels);
    Ok(stream)
}

/// Best-effort resample to 16kHz. For v1 we use linear interpolation;
/// replace with proper SincFixedIn if quality is insufficient.
pub fn resample_to_16k(input: &[f32], input_rate: u32) -> Vec<f32> {
    if input_rate == 16000 || input.is_empty() {
        return input.to_vec();
    }
    let ratio = 16000.0 / input_rate as f64;
    let out_len = (input.len() as f64 * ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_idx = i as f64 / ratio;
        let i0 = src_idx.floor() as usize;
        let i1 = (i0 + 1).min(input.len() - 1);
        let t = (src_idx - i0 as f64) as f32;
        out.push(input[i0] * (1.0 - t) + input[i1] * t);
    }
    out
}

pub fn encode_wav(pcm: &[f32]) -> Result<Vec<u8>> {
    let mut cursor = std::io::Cursor::new(Vec::<u8>::new());
    {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 16000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::new(&mut cursor, spec)?;
        for &s in pcm {
            let clamped = s.clamp(-1.0, 1.0);
            writer.write_sample((clamped * 32767.0) as i16)?;
        }
        writer.finalize()?;
    }
    Ok(cursor.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;
    use hound::WavReader;

    #[test]
    fn resample_44100_to_16000() {
        let input: Vec<f32> = vec![0.0; 44100];
        let out = resample_to_16k(&input, 44100);
        assert!((out.len() as i32 - 16000).abs() < 800, "got {}", out.len());
    }

    #[test]
    fn resample_passthrough_when_already_16k() {
        let input: Vec<f32> = vec![0.1; 16000];
        let out = resample_to_16k(&input, 16000);
        assert_eq!(out.len(), 16000);
    }

    #[test]
    fn encode_wav_produces_valid_header() {
        let pcm: Vec<f32> = vec![0.0; 16000];
        let wav = encode_wav(&pcm).unwrap();
        assert_eq!(&wav[0..4], b"RIFF");
        assert_eq!(&wav[8..12], b"WAVE");
        let mut reader = WavReader::new(std::io::Cursor::new(wav)).unwrap();
        assert_eq!(reader.spec().sample_rate, 16000);
        assert_eq!(reader.spec().channels, 1);
    }
}