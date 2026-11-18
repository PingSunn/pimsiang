use std::sync::{Arc, Mutex};

use engine::{
    Engine, EngineError, LanguageMode, MIN_SILENCE_SAMPLES, Recognizer, SPEECH_PAD_SAMPLES,
    SpeechDetector, StreamingEngine, UtteranceOptions,
};

const SECOND: usize = 16_000;

#[derive(Clone, Default)]
struct Calls(Arc<Mutex<Vec<usize>>>);

impl Calls {
    fn lengths(&self) -> Vec<usize> {
        self.0.lock().unwrap().clone()
    }
}

struct TaggingRecognizer {
    loaded: bool,
    calls: Calls,
}

impl Recognizer for TaggingRecognizer {
    fn load(&mut self) -> Result<(), EngineError> {
        self.loaded = true;
        Ok(())
    }

    fn unload(&mut self) {
        self.loaded = false;
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn recognize(&mut self, samples: &[f32], _: LanguageMode) -> Result<String, EngineError> {
        self.calls.0.lock().unwrap().push(samples.len());
        let loudest = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        Ok(if loudest > 0.0 {
            format!("w{}", (loudest * 100.0).round() as u32)
        } else {
            String::new()
        })
    }
}

struct EnergyDetector {
    loaded: bool,
}

impl SpeechDetector for EnergyDetector {
    fn load(&mut self) -> Result<(), EngineError> {
        self.loaded = true;
        Ok(())
    }

    fn unload(&mut self) {
        self.loaded = false;
    }

    fn is_loaded(&self) -> bool {
        self.loaded
    }

    fn reset(&mut self) {}

    fn speech_probability(&mut self, window: &[f32]) -> Result<f32, EngineError> {
        Ok(if window.iter().any(|s| s.abs() > 0.05) {
            1.0
        } else {
            0.0
        })
    }
}

fn engine() -> (StreamingEngine<TaggingRecognizer, EnergyDetector>, Calls) {
    let calls = Calls::default();
    let mut engine = StreamingEngine::with_detector(
        TaggingRecognizer {
            loaded: false,
            calls: calls.clone(),
        },
        EnergyDetector { loaded: false },
    );
    engine.load().unwrap();
    engine.start_utterance(UtteranceOptions::default()).unwrap();
    (engine, calls)
}

fn speech(level: f32, seconds: f32) -> Vec<f32> {
    vec![level; (seconds * SECOND as f32) as usize]
}

fn silence(seconds: f32) -> Vec<f32> {
    vec![0.0; (seconds * SECOND as f32) as usize]
}

fn stream(engine: &mut impl Engine, audio: &[f32]) -> Vec<String> {
    audio
        .chunks(1_600)
        .filter_map(|chunk| engine.push_audio(chunk).unwrap())
        .collect()
}

#[test]
fn final_text_joins_every_segment_in_order() {
    let (mut engine, _) = engine();
    let audio = [
        speech(0.2, 1.0),
        silence(1.0),
        speech(0.3, 1.0),
        silence(1.0),
        speech(0.4, 1.0),
    ]
    .concat();

    stream(&mut engine, &audio);

    assert_eq!(engine.finish_utterance().unwrap(), "w20 w30 w40");
}

#[test]
fn a_long_pause_loses_nothing_and_is_never_decoded() {
    let (mut engine, calls) = engine();
    let audio = [speech(0.2, 1.0), silence(45.0), speech(0.3, 1.0)].concat();

    stream(&mut engine, &audio);

    assert_eq!(engine.finish_utterance().unwrap(), "w20 w30");
    let longest = calls.lengths().into_iter().max().unwrap();
    assert!(longest <= SECOND + 2 * SPEECH_PAD_SAMPLES + MIN_SILENCE_SAMPLES);
}

#[test]
fn closed_segments_are_locked_and_never_decoded_again() {
    let (mut engine, calls) = engine();
    let mut audio = Vec::new();
    for _ in 0..10 {
        audio.extend(speech(0.2, 1.0));
        audio.extend(silence(1.0));
    }

    stream(&mut engine, &audio);
    engine.finish_utterance().unwrap();

    let longest = calls.lengths().into_iter().max().unwrap();
    assert!(
        longest <= SECOND + 2 * SPEECH_PAD_SAMPLES + MIN_SILENCE_SAMPLES,
        "decoded {longest} samples at once from a 20 s Utterance"
    );
}

#[test]
fn partial_text_carries_locked_segments_and_the_open_one() {
    let (mut engine, _) = engine();
    let first = [speech(0.2, 1.0), silence(1.0)].concat();
    stream(&mut engine, &first);

    let partials = stream(&mut engine, &speech(0.3, 1.0));

    assert_eq!(partials.last().map(String::as_str), Some("w20 w30"));
}

#[test]
fn silence_only_input_has_empty_final_text_and_decodes_nothing() {
    let (mut engine, calls) = engine();

    let partials = stream(&mut engine, &silence(5.0));

    assert!(partials.is_empty());
    assert_eq!(engine.finish_utterance().unwrap(), "");
    assert!(calls.lengths().is_empty());
}

#[test]
fn leading_silence_is_not_decoded() {
    let (mut engine, calls) = engine();

    stream(&mut engine, &[silence(20.0), speech(0.2, 1.0)].concat());
    let text = engine.finish_utterance().unwrap();

    assert_eq!(text, "w20");
    let longest = calls.lengths().into_iter().max().unwrap();
    assert!(longest <= SECOND + 2 * SPEECH_PAD_SAMPLES);
}

#[test]
fn a_short_pause_inside_a_phrase_does_not_split_it() {
    let (mut engine, calls) = engine();
    let audio = [speech(0.2, 1.0), silence(0.3), speech(0.2, 1.0)].concat();

    stream(&mut engine, &audio);
    engine.finish_utterance().unwrap();

    assert!(calls.lengths().into_iter().any(|n| n >= 2 * SECOND));
}
