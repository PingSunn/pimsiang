use engine::{Engine, EngineError, LanguageMode, Recognizer, StreamingEngine, UtteranceOptions};

#[derive(Default)]
struct CountingRecognizer {
    loaded: bool,
}

impl Recognizer for CountingRecognizer {
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

    fn recognize(
        &mut self,
        samples: &[f32],
        language: LanguageMode,
    ) -> Result<String, EngineError> {
        Ok(format!("{language} {}", samples.len()))
    }
}

fn loaded_engine() -> StreamingEngine<CountingRecognizer> {
    let mut engine = StreamingEngine::with_recognizer(CountingRecognizer::default());
    engine.load().unwrap();
    engine
}

const TICK: usize = 8_000;

#[test]
fn partial_text_arrives_every_500ms_of_audio_then_one_final_text() {
    let mut engine = loaded_engine();
    engine.start_utterance(UtteranceOptions::default()).unwrap();

    let mut partials = Vec::new();
    for _ in 0..5 {
        if let Some(text) = engine.push_audio(&[0.0; 3_200]).unwrap() {
            partials.push(text);
        }
    }
    let final_text = engine.finish_utterance().unwrap();

    assert_eq!(partials, vec!["th-mix 9600"]);
    assert_eq!(final_text, "th-mix 16000");
}

#[test]
fn partial_text_re_decodes_all_buffered_audio() {
    let mut engine = loaded_engine();
    engine.start_utterance(UtteranceOptions::default()).unwrap();

    let first = engine.push_audio(&[0.0; TICK]).unwrap();
    let second = engine.push_audio(&[0.0; TICK]).unwrap();

    assert_eq!(first.as_deref(), Some("th-mix 8000"));
    assert_eq!(second.as_deref(), Some("th-mix 16000"));
}

#[test]
fn language_mode_is_an_utterance_option() {
    let mut engine = loaded_engine();
    engine
        .start_utterance(UtteranceOptions {
            language: LanguageMode::ThTh,
        })
        .unwrap();
    engine.push_audio(&[0.0; 100]).unwrap();

    assert_eq!(engine.finish_utterance().unwrap(), "th-TH 100");
}

#[test]
fn an_utterance_without_audio_has_empty_final_text() {
    let mut engine = loaded_engine();
    engine.start_utterance(UtteranceOptions::default()).unwrap();

    assert_eq!(engine.finish_utterance().unwrap(), "");
}

#[test]
fn a_new_utterance_starts_with_no_audio_from_the_previous_one() {
    let mut engine = loaded_engine();
    engine.start_utterance(UtteranceOptions::default()).unwrap();
    engine.push_audio(&[0.0; 500]).unwrap();
    engine.cancel_utterance();

    engine.start_utterance(UtteranceOptions::default()).unwrap();
    engine.push_audio(&[0.0; 200]).unwrap();

    assert_eq!(engine.finish_utterance().unwrap(), "th-mix 200");
}

#[test]
fn audio_outside_an_utterance_is_rejected() {
    let mut engine = loaded_engine();

    assert!(matches!(
        engine.push_audio(&[0.0; 10]),
        Err(EngineError::NoUtterance)
    ));
    assert!(matches!(
        engine.finish_utterance(),
        Err(EngineError::NoUtterance)
    ));
}

#[test]
fn an_utterance_needs_the_model_loaded() {
    let mut engine = StreamingEngine::with_recognizer(CountingRecognizer::default());

    assert!(!engine.is_loaded());
    assert!(matches!(
        engine.start_utterance(UtteranceOptions::default()),
        Err(EngineError::NotLoaded)
    ));

    engine.load().unwrap();
    assert!(engine.is_loaded());
    engine.unload();
    assert!(!engine.is_loaded());
}

#[test]
fn unloading_ends_the_open_utterance() {
    let mut engine = loaded_engine();
    engine.start_utterance(UtteranceOptions::default()).unwrap();
    engine.unload();

    assert!(matches!(
        engine.push_audio(&[0.0; 10]),
        Err(EngineError::NoUtterance)
    ));
}

#[test]
fn language_modes_parse_from_their_names() {
    assert_eq!(
        "th-mix".parse::<LanguageMode>().unwrap(),
        LanguageMode::ThMix
    );
    assert_eq!("th-TH".parse::<LanguageMode>().unwrap(), LanguageMode::ThTh);
    assert!("en-US".parse::<LanguageMode>().is_err());
    assert_eq!(LanguageMode::default(), LanguageMode::ThMix);
    assert_eq!(LanguageMode::ThMix.prompt_index(), 63);
    assert_eq!(LanguageMode::ThTh.prompt_index(), 32);
}
