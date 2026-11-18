use engine::{EngineError, RecognitionMode};

const SHORT: usize = 2 * 16_000 - 1;
const LONG: usize = 2 * 16_000;

fn run(mode: RecognitionMode, samples: usize, outputs: &[&str]) -> (String, Vec<f32>) {
    let mut biases = Vec::new();
    let mut outputs = outputs.iter();
    let text = mode
        .decode(samples, |bias| -> Result<String, EngineError> {
            biases.push(bias);
            Ok(outputs.next().map_or_else(String::new, |t| (*t).to_owned()))
        })
        .unwrap();
    (text, biases)
}

#[test]
fn standard_decodes_a_long_clip_once_at_minus_one() {
    let (text, biases) = run(RecognitionMode::Standard, LONG, &["สวัสดี"]);

    assert_eq!(text, "สวัสดี");
    assert_eq!(biases, vec![-1.0]);
}

#[test]
fn standard_decodes_a_short_clip_again_at_minus_two_and_a_half() {
    let (text, biases) = run(RecognitionMode::Standard, SHORT, &["ก่อน", "หลัง"]);

    assert_eq!(text, "หลัง");
    assert_eq!(biases, vec![-1.0, -2.5]);
}

#[test]
fn standard_decodes_empty_output_again_at_minus_four() {
    let (text, biases) = run(RecognitionMode::Standard, LONG, &["", "พบ"]);

    assert_eq!(text, "พบ");
    assert_eq!(biases, vec![-1.0, -4.0]);
}

#[test]
fn standard_retries_a_short_clip_that_is_still_empty_at_minus_four() {
    let (text, biases) = run(RecognitionMode::Standard, SHORT, &["", "", "พบ"]);

    assert_eq!(text, "พบ");
    assert_eq!(biases, vec![-1.0, -2.5, -4.0]);
}

#[test]
fn conversation_decodes_once_at_minus_two_whatever_the_length() {
    for samples in [SHORT, LONG] {
        let (text, biases) = run(RecognitionMode::Conversation, samples, &["คุย"]);

        assert_eq!(text, "คุย");
        assert_eq!(biases, vec![-2.0]);
    }
}

#[test]
fn conversation_decodes_empty_output_again_at_minus_four() {
    let (text, biases) = run(RecognitionMode::Conversation, SHORT, &["", "พบ"]);

    assert_eq!(text, "พบ");
    assert_eq!(biases, vec![-2.0, -4.0]);
}

#[test]
fn a_decode_error_stops_the_policy() {
    let result = RecognitionMode::Standard.decode(LONG, |_| Err(EngineError::NotLoaded));

    assert!(matches!(result, Err(EngineError::NotLoaded)));
}

#[test]
fn recognition_mode_parses_and_prints_its_names() {
    assert_eq!(
        "standard".parse::<RecognitionMode>().unwrap(),
        RecognitionMode::Standard
    );
    assert_eq!(
        "conversation".parse::<RecognitionMode>().unwrap(),
        RecognitionMode::Conversation
    );
    assert!("fast".parse::<RecognitionMode>().is_err());
    assert_eq!(RecognitionMode::default(), RecognitionMode::Standard);
    assert_eq!(RecognitionMode::Conversation.to_string(), "conversation");
}
