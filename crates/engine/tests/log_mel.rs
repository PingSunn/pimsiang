use std::f64::consts::PI;

use engine::log_mel;

fn two_tones() -> Vec<f32> {
    (0..4000)
        .map(|i| {
            let t = f64::from(i) / 16_000.0;
            (0.3 * (2.0 * PI * 440.0 * t).sin() + 0.2 * (2.0 * PI * 3000.0 * t).sin()) as f32
        })
        .collect()
}

#[test]
fn has_128_bands_and_one_frame_per_10ms_plus_one() {
    let features = log_mel(&two_tones());

    assert_eq!(features.bands, 128);
    assert_eq!(features.frames, 26);
    assert_eq!(features.values.len(), 128 * 26);
}

#[test]
fn matches_the_reference_front_end() {
    let reference: [(usize, usize, f64); 10] = [
        (0, 0, -9.11912),
        (5, 0, -7.62115),
        (10, 12, -12.34048),
        (20, 12, -3.04436),
        (62, 12, -16.62922),
        (63, 12, -16.62534),
        (100, 12, -16.59453),
        (127, 25, -8.69976),
        (40, 25, -6.53535),
        (10, 1, -10.18810),
    ];

    let features = log_mel(&two_tones());

    for (band, frame, expected) in reference {
        let actual = f64::from(features.value(band, frame));
        assert!(
            (actual - expected).abs() < 0.01,
            "band {band} frame {frame}: {actual} != {expected}"
        );
    }
}

#[test]
fn silence_sits_at_the_log_floor() {
    let features = log_mel(&vec![0.0; 1600]);

    let floor = (2.0f32).powi(-24).ln();
    assert!(features.values.iter().all(|v| (v - floor).abs() < 1e-3));
}
