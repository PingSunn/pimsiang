use engine::fake::FakeEngine;
use engine::{Engine, EngineError, LanguageMode, UtteranceOptions};

fn drive(engine: &mut dyn Engine, pushes: usize) -> (Vec<String>, String) {
    engine.load().unwrap();
    engine
        .start_utterance(UtteranceOptions {
            language: LanguageMode::ThTh,
            ..UtteranceOptions::default()
        })
        .unwrap();
    let partials = (0..pushes)
        .filter_map(|_| engine.push_audio(&[0.0; 160]).unwrap())
        .collect();
    (partials, engine.finish_utterance().unwrap())
}

#[test]
fn returns_scripted_partial_then_final_text_through_the_engine_interface() {
    let mut fake = FakeEngine::new(["สวัส", "สวัสดี"], "สวัสดีครับ");

    let (partials, final_text) = drive(&mut fake, 4);

    assert_eq!(partials, vec!["สวัส", "สวัสดี"]);
    assert_eq!(final_text, "สวัสดีครับ");
}

#[test]
fn records_what_the_caller_asked_for() {
    let mut fake = FakeEngine::new(Vec::<String>::new(), "");

    drive(&mut fake, 3);

    assert_eq!(
        fake.last_options().map(|o| o.language),
        Some(LanguageMode::ThTh)
    );
    assert_eq!(fake.pushed_samples(), 480);
    assert_eq!(fake.load_count(), 1);
}

#[test]
fn follows_the_same_contract_as_the_real_engine() {
    let mut fake = FakeEngine::new(["a"], "b");

    assert!(matches!(
        fake.start_utterance(UtteranceOptions::default()),
        Err(EngineError::NotLoaded)
    ));
    fake.load().unwrap();
    assert!(matches!(
        fake.push_audio(&[0.0]),
        Err(EngineError::NoUtterance)
    ));
    fake.start_utterance(UtteranceOptions::default()).unwrap();
    fake.cancel_utterance();
    assert!(matches!(
        fake.finish_utterance(),
        Err(EngineError::NoUtterance)
    ));
    fake.unload();
    assert!(!fake.is_loaded());
}
