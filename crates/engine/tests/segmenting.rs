use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use engine::{
    Engine, EngineError, LanguageMode, MAX_SEGMENT_SAMPLES, MIN_SILENCE_SAMPLES, Recognizer,
    SPEECH_PAD_SAMPLES, SpeechDetector, StreamingEngine, UtteranceOptions, WINDOW_SAMPLES,
};

const SECOND: usize = 16_000;

#[derive(Clone, Default)]
struct Calls(Arc<Mutex<Vec<usize>>>);

impl Calls {
    fn lengths(&self) -> Vec<usize> {
        self.0.lock().unwrap().clone()
    }
}

#[derive(Clone, Default)]
struct FailOnce(Arc<AtomicBool>);

impl FailOnce {
    fn arm(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn take(&self) -> bool {
        self.0.swap(false, Ordering::SeqCst)
    }
}

struct TaggingRecognizer {
    loaded: bool,
    calls: Calls,
    failure: FailOnce,
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
        if self.failure.take() {
            return Err(EngineError::Model("injected failure".into()));
        }
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
    failure: FailOnce,
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
        if self.failure.take() {
            return Err(EngineError::Model("injected failure".into()));
        }
        Ok(if window.iter().any(|s| s.abs() > 0.05) {
            1.0
        } else {
            0.0
        })
    }
}

fn flaky_engine() -> (
    StreamingEngine<TaggingRecognizer, EnergyDetector>,
    Calls,
    FailOnce,
    FailOnce,
) {
    let calls = Calls::default();
    let recognizer_failure = FailOnce::default();
    let detector_failure = FailOnce::default();
    let mut engine = StreamingEngine::with_detector(
        TaggingRecognizer {
            loaded: false,
            calls: calls.clone(),
            failure: recognizer_failure.clone(),
        },
        EnergyDetector {
            loaded: false,
            failure: detector_failure.clone(),
        },
    );
    engine.load().unwrap();
    engine.start_utterance(UtteranceOptions::default()).unwrap();
    (engine, calls, recognizer_failure, detector_failure)
}

fn engine() -> (StreamingEngine<TaggingRecognizer, EnergyDetector>, Calls) {
    let (engine, calls, _, _) = flaky_engine();
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

#[test]
fn finish_recovers_after_a_failed_detector_call_mid_push() {
    let (mut engine, _, _, detector_failure) = flaky_engine();
    detector_failure.arm();

    let result = engine.push_audio(&[speech(0.2, 1.0), silence(2.0)].concat());

    assert!(result.is_err());
    assert_eq!(engine.finish_utterance().unwrap(), "w20");
}

#[test]
fn finish_recovers_after_a_failed_segment_close_mid_push() {
    let (mut engine, _, recognizer_failure, _) = flaky_engine();
    recognizer_failure.arm();

    let result = engine.push_audio(&[speech(0.2, 1.0), silence(2.0)].concat());

    assert!(result.is_err());
    assert_eq!(engine.finish_utterance().unwrap(), "w20");
}

#[test]
fn audio_after_a_failed_push_is_neither_lost_nor_duplicated() {
    let (mut engine, _, recognizer_failure, _) = flaky_engine();
    recognizer_failure.arm();

    let first = engine.push_audio(&[speech(0.2, 1.0), silence(2.0)].concat());
    stream(&mut engine, &[speech(0.3, 1.0), silence(1.0)].concat());

    assert!(first.is_err());
    assert_eq!(engine.finish_utterance().unwrap(), "w20 w30");
}

#[test]
fn push_tail_yields_the_same_final_text_as_push_audio_without_a_partial_decode() {
    let audio = [speech(0.2, 1.0), silence(1.0), speech(0.3, 1.0)].concat();
    let (mut streamed, _) = engine();
    stream(&mut streamed, &audio);
    let expected = streamed.finish_utterance().unwrap();

    let (mut tailed, calls) = engine();
    tailed.push_tail(&audio).unwrap();
    let decodes_before_finish = calls.lengths().len();
    let actual = tailed.finish_utterance().unwrap();

    assert_eq!(actual, expected);
    assert_eq!(decodes_before_finish, 1);
}

struct SpanRecognizer {
    calls: Calls,
}

impl Recognizer for SpanRecognizer {
    fn load(&mut self) -> Result<(), EngineError> {
        Ok(())
    }

    fn unload(&mut self) {}

    fn is_loaded(&self) -> bool {
        true
    }

    fn recognize(&mut self, samples: &[f32], _: LanguageMode) -> Result<String, EngineError> {
        self.calls.0.lock().unwrap().push(samples.len());
        let index = |sample: f32| (sample / INDEX_STEP).round() as usize - 1;
        Ok(match (samples.first(), samples.last()) {
            (Some(&first), Some(&last)) => format!("{}:{}|", index(first), index(last) + 1),
            _ => String::new(),
        })
    }
}

const INDEX_STEP: f32 = 1e-6;

struct ScriptedDetector {
    probabilities: fn(usize) -> f32,
    next: usize,
}

impl SpeechDetector for ScriptedDetector {
    fn load(&mut self) -> Result<(), EngineError> {
        Ok(())
    }

    fn unload(&mut self) {}

    fn is_loaded(&self) -> bool {
        true
    }

    fn reset(&mut self) {
        self.next = 0;
    }

    fn speech_probability(&mut self, _: &[f32]) -> Result<f32, EngineError> {
        let probability = (self.probabilities)(self.next);
        self.next += 1;
        Ok(probability)
    }
}

fn scripted_engine(
    probabilities: fn(usize) -> f32,
) -> (StreamingEngine<SpanRecognizer, ScriptedDetector>, Calls) {
    let calls = Calls::default();
    let mut engine = StreamingEngine::with_detector(
        SpanRecognizer {
            calls: calls.clone(),
        },
        ScriptedDetector {
            probabilities,
            next: 0,
        },
    );
    engine.load().unwrap();
    engine.start_utterance(UtteranceOptions::default()).unwrap();
    (engine, calls)
}

fn indexed_audio(windows: usize) -> Vec<f32> {
    (0..windows * WINDOW_SAMPLES)
        .map(|i| (i + 1) as f32 * INDEX_STEP)
        .collect()
}

fn spans(text: &str) -> Vec<(usize, usize)> {
    text.split('|')
        .filter(|span| !span.trim().is_empty())
        .map(|span| {
            let (start, end) = span.trim().split_once(':').unwrap();
            (start.parse().unwrap(), end.parse().unwrap())
        })
        .collect()
}

fn continuous_speech_with_dips(window: usize) -> f32 {
    match window {
        300 => 0.0,
        430 => 0.3,
        440 => 0.2,
        _ => 0.9,
    }
}

#[test]
fn a_segment_reaching_the_cap_is_cut_at_the_lowest_probability_window_near_its_end() {
    let (mut engine, _) = scripted_engine(continuous_speech_with_dips);

    stream(&mut engine, &indexed_audio(480));
    let text = engine.finish_utterance().unwrap();

    let cut = 440 * WINDOW_SAMPLES + WINDOW_SAMPLES / 2;
    assert_eq!(spans(&text)[0], (0, cut));
}

#[test]
fn forced_cuts_lose_and_duplicate_no_audio() {
    let windows = 1_250;
    let (mut engine, _) = scripted_engine(|_| 0.9);

    stream(&mut engine, &indexed_audio(windows));
    let text = engine.finish_utterance().unwrap();

    let spans = spans(&text);
    assert!(spans.len() >= 3, "{text}");
    assert_eq!(spans.first().map(|s| s.0), Some(0));
    assert_eq!(spans.last().map(|s| s.1), Some(windows * WINDOW_SAMPLES));
    assert!(
        spans.windows(2).all(|pair| pair[0].1 == pair[1].0),
        "{text}"
    );
    assert!(
        spans
            .iter()
            .all(|(start, end)| end - start <= MAX_SEGMENT_SAMPLES + WINDOW_SAMPLES)
    );
}

#[test]
fn a_forced_cut_joins_without_a_space_but_a_silence_close_joins_with_one() {
    let (mut engine, _) = scripted_engine(|window| {
        if (600..660).contains(&window) {
            0.0
        } else {
            0.9
        }
    });

    stream(&mut engine, &indexed_audio(1_000));
    let text = engine.finish_utterance().unwrap();

    let phrases: Vec<&str> = text.split(' ').collect();
    assert_eq!(phrases.len(), 2, "{text}");
    assert_eq!(spans(phrases[0]).len(), 2, "{text}");
    assert_eq!(spans(phrases[1]).len(), 1, "{text}");
}

#[test]
fn partial_text_decodes_at_most_one_capped_segment_per_tick() {
    let (mut engine, calls) = scripted_engine(|_| 0.9);
    let audio = indexed_audio(1_250);

    for chunk in audio.chunks(1_600) {
        let before = calls.lengths().len();
        engine.push_audio(chunk).unwrap();
        let decoded = calls.lengths()[before..].to_vec();
        assert!(decoded.len() <= 1, "decoded {decoded:?} in one tick");
        assert!(
            decoded
                .iter()
                .all(|&n| n <= MAX_SEGMENT_SAMPLES + WINDOW_SAMPLES)
        );
    }
    engine.finish_utterance().unwrap();
}
