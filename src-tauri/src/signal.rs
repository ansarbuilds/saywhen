//! Microphone frame helpers. The capture thread calls these before voice detection.

pub const TARGET_SAMPLE_RATE: u32 = 16_000;

pub fn downmix_to_mono_f32(data: &[f32], channels: usize) -> Vec<f32> {
    let ch = channels.max(1);
    if ch == 1 {
        return data.to_vec();
    }
    data.chunks(ch)
        .map(|frame| frame.iter().copied().sum::<f32>() / frame.len() as f32)
        .collect()
}

/// Linear resample to 16 kHz. An empty input or a zero-length result stays empty.
pub fn resample_to_16k(input: &[f32], in_rate: u32) -> Vec<f32> {
    if in_rate == TARGET_SAMPLE_RATE || input.is_empty() {
        return input.to_vec();
    }
    let ratio = in_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let out_len = ((input.len() as f64) / ratio).floor() as usize;
    if out_len == 0 {
        return Vec::new();
    }
    let mut output = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src_pos = i as f64 * ratio;
        let idx = src_pos.floor() as usize;
        let frac = (src_pos - idx as f64) as f32;
        let s0 = input[idx.min(input.len() - 1)];
        let s1 = input[(idx + 1).min(input.len() - 1)];
        output.push(s0 + frac * (s1 - s0));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stereo_downmix_averages_each_frame() {
        let mono = downmix_to_mono_f32(&[0.0, 1.0, 0.5, 0.5], 2);
        assert_eq!(mono, vec![0.5, 0.5]);
        assert_eq!(downmix_to_mono_f32(&[0.2, 0.4], 1), vec![0.2, 0.4]);
    }

    #[test]
    fn resample_keeps_16k_and_shrinks_48k() {
        let samples = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0];
        assert_eq!(resample_to_16k(&samples, 16_000), samples);
        let down = resample_to_16k(&samples, 48_000);
        assert_eq!(down.len(), 2);
        assert!(resample_to_16k(&[], 48_000).is_empty());
    }
}
