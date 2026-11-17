use std::f32::consts::PI;
use std::path::Path;

fn write_wav(path: &Path, sample_rate: u32, channels: u16, frames: &[f32]) {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec).unwrap();
    for &s in frames {
        for _ in 0..channels {
            writer.write_sample((s * 32767.0) as i16).unwrap();
        }
    }
    writer.finalize().unwrap();
}

fn sine(freq: f32, sample_rate: u32, seconds: f32) -> Vec<f32> {
    let n = (sample_rate as f32 * seconds) as usize;
    (0..n)
        .map(|i| 0.5 * (2.0 * PI * freq * i as f32 / sample_rate as f32).sin())
        .collect()
}

fn zero_crossings(samples: &[f32]) -> usize {
    samples
        .windows(2)
        .filter(|w| (w[0] < 0.0) != (w[1] < 0.0))
        .count()
}

#[test]
fn reads_16khz_mono_wav_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("a.wav");
    let tone = sine(440.0, 16_000, 1.0);
    write_wav(&path, 16_000, 1, &tone);

    let audio = audio::read_wav_16k_mono(&path).unwrap();

    assert_eq!(audio.len(), 16_000);
    for (a, b) in audio.iter().zip(&tone) {
        assert!((a - b).abs() < 1e-3);
    }
}

#[test]
fn resamples_44khz_stereo_to_16khz_mono_keeping_pitch() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("b.wav");
    write_wav(&path, 44_100, 2, &sine(1_000.0, 44_100, 1.0));

    let audio = audio::read_wav_16k_mono(&path).unwrap();

    assert!(
        (audio.len() as i64 - 16_000).abs() <= 1,
        "len {}",
        audio.len()
    );
    let crossings = zero_crossings(&audio[800..15_200]);
    assert!(
        (1_780..=1_820).contains(&crossings),
        "crossings {crossings}"
    );
    let peak = audio[800..15_200]
        .iter()
        .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!((0.45..=0.55).contains(&peak), "peak {peak}");
}

#[test]
fn removes_tones_above_the_new_nyquist() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("c.wav");
    write_wav(&path, 48_000, 1, &sine(12_000.0, 48_000, 1.0));

    let audio = audio::read_wav_16k_mono(&path).unwrap();

    let peak = audio[800..15_200]
        .iter()
        .fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak < 0.05, "peak {peak}");
}

#[test]
fn reports_a_missing_file() {
    assert!(audio::read_wav_16k_mono(Path::new("/nonexistent/x.wav")).is_err());
}
