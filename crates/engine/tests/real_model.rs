use std::path::{Path, PathBuf};

use engine::{Engine, ModelFiles, OnnxEngine, Precision, UtteranceOptions, character_error_rate};

const CER_THRESHOLD: f64 = 0.10;

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn model_files() -> ModelFiles {
    let dir = std::env::var_os("PIMSIANG_MODEL_DIR")
        .expect("set PIMSIANG_MODEL_DIR to the folder holding the unzipped Model");
    let precision = std::env::var("PIMSIANG_MODEL_PRECISION")
        .unwrap_or_else(|_| "int8".to_owned())
        .parse::<Precision>()
        .unwrap();
    ModelFiles::new(dir, precision)
}

fn fixtures() -> Vec<(String, String)> {
    std::fs::read_to_string(fixtures_dir().join("transcripts.tsv"))
        .unwrap()
        .lines()
        .filter_map(|line| line.split_once('\t'))
        .map(|(name, text)| (name.to_owned(), text.to_owned()))
        .collect()
}

fn read_fixture(name: &str) -> Vec<f32> {
    let path = fixtures_dir().join(format!("{name}.wav"));
    let mut reader = hound::WavReader::open(&path)
        .unwrap_or_else(|e| panic!("{}: {e}; run scripts/make-fixtures.sh", path.display()));
    assert_eq!(reader.spec().sample_rate, 16_000);
    reader
        .samples::<i16>()
        .map(|s| f32::from(s.unwrap()) / 32_768.0)
        .collect()
}

fn loaded_engine() -> OnnxEngine {
    let mut engine = OnnxEngine::new(model_files());
    engine.load().unwrap();
    engine
}

#[test]
#[ignore = "needs the Model on disk: set PIMSIANG_MODEL_DIR"]
fn thai_fixtures_produce_final_text_under_the_cer_threshold() {
    let mut engine = loaded_engine();

    for (name, reference) in fixtures() {
        engine.start_utterance(UtteranceOptions::default()).unwrap();
        engine.push_audio(&read_fixture(&name)).unwrap();
        let final_text = engine.finish_utterance().unwrap();

        let cer = character_error_rate(&reference, &final_text);
        println!("{name}: CER {cer:.3} -> {final_text}");
        assert!(
            cer <= CER_THRESHOLD,
            "{name}: CER {cer:.3} for `{final_text}`"
        );
        assert!(
            !final_text.contains("\u{0E4D}\u{0E32}"),
            "{name}: sara am written as nikhahit + sara aa in `{final_text}`"
        );
    }
}

#[test]
#[ignore = "needs the Model on disk: set PIMSIANG_MODEL_DIR"]
fn partial_text_arrives_before_final_text_while_streaming() {
    let mut engine = loaded_engine();
    let audio = read_fixture("meeting");

    engine.start_utterance(UtteranceOptions::default()).unwrap();
    let partials: Vec<String> = audio
        .chunks(1_600)
        .filter_map(|chunk| engine.push_audio(chunk).unwrap())
        .collect();
    let final_text = engine.finish_utterance().unwrap();

    assert!(partials.len() >= 3, "only {} Partial texts", partials.len());
    assert!(partials.iter().any(|p| !p.is_empty()));
    assert!(!final_text.is_empty());
}

fn reference(name: &str) -> String {
    fixtures()
        .into_iter()
        .find(|(fixture, _)| fixture == name)
        .map(|(_, text)| text)
        .unwrap_or_else(|| panic!("no transcript for {name}"))
}

#[test]
#[ignore = "needs the Model and Silero VAD on disk: set PIMSIANG_MODEL_DIR"]
fn a_long_pause_in_the_middle_loses_no_words_after_it() {
    let mut engine = loaded_engine();
    let audio = [
        read_fixture("meeting"),
        vec![0.0; 45 * 16_000],
        read_fixture("food"),
    ]
    .concat();

    engine.start_utterance(UtteranceOptions::default()).unwrap();
    for chunk in audio.chunks(1_600) {
        engine.push_audio(chunk).unwrap();
    }
    let final_text = engine.finish_utterance().unwrap();

    let expected = format!("{} {}", reference("meeting"), reference("food"));
    let cer = character_error_rate(&expected, &final_text);
    println!("CER {cer:.3} -> {final_text}");
    assert!(cer <= CER_THRESHOLD, "CER {cer:.3} for `{final_text}`");
}

#[test]
#[ignore = "needs the Model and Silero VAD on disk: set PIMSIANG_MODEL_DIR"]
fn silence_only_input_has_empty_final_text() {
    let mut engine = loaded_engine();

    engine.start_utterance(UtteranceOptions::default()).unwrap();
    for chunk in vec![0.0f32; 5 * 16_000].chunks(1_600) {
        assert_eq!(engine.push_audio(chunk).unwrap(), None);
    }

    assert_eq!(engine.finish_utterance().unwrap(), "");
}
