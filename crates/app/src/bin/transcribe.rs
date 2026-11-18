use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use engine::{Engine, LanguageMode, ModelFiles, OnnxEngine, Precision, UtteranceOptions};

const CHUNK_SAMPLES: usize = 1_600;
const SAMPLE_RATE: f64 = audio::TARGET_SAMPLE_RATE as f64;

#[derive(Parser)]
#[command(about = "Print the Final text of a Thai WAV recording")]
struct Args {
    wav: PathBuf,
    #[arg(long, env = "PIMSIANG_MODEL_DIR")]
    model_dir: PathBuf,
    #[arg(long, default_value = "int8")]
    precision: Precision,
    #[arg(long, default_value = "th-mix")]
    language: LanguageMode,
    #[arg(short, long)]
    verbose: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let samples = audio::read_wav_16k_mono(&args.wav)
        .with_context(|| format!("reading {}", args.wav.display()))?;

    let mut engine = OnnxEngine::new(ModelFiles::new(&args.model_dir, args.precision));
    let load_started = Instant::now();
    engine.load().context("loading the Model")?;
    if args.verbose {
        eprintln!(
            "loaded {} Model in {:.2} s",
            args.precision,
            load_started.elapsed().as_secs_f64()
        );
    }

    let started = Instant::now();
    engine.start_utterance(UtteranceOptions {
        language: args.language,
    })?;
    let mut pushed = 0usize;
    for chunk in samples.chunks(CHUNK_SAMPLES) {
        pushed += chunk.len();
        let at = pushed as f64 / SAMPLE_RATE;
        match engine.push_audio(chunk) {
            Ok(Some(partial)) if args.verbose => eprintln!("[partial {at:6.2} s] {partial}"),
            Ok(_) => {}
            Err(error) => eprintln!("warning: Partial text failed at {at:.2} s: {error}"),
        }
    }
    let final_started = Instant::now();
    let final_text = engine.finish_utterance()?;
    let final_elapsed = final_started.elapsed().as_secs_f64();
    let elapsed = started.elapsed().as_secs_f64();

    if args.verbose {
        let duration = (samples.len() as f64 / SAMPLE_RATE).max(f64::EPSILON);
        eprintln!(
            "[final   {duration:6.2} s] Final text decoded in {final_elapsed:.2} s (real-time factor {:.3}); \
             whole Utterance with Partial text took {elapsed:.2} s",
            final_elapsed / duration
        );
    }
    println!("{final_text}");
    Ok(())
}
