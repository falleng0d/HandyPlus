use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use hound::{SampleFormat, WavReader, WavSpec};
use rubato::{FftFixedIn, Resampler};
use std::path::PathBuf;
use std::time::Instant;
use transcribe_rs::accel::{
    set_ort_accelerator, set_whisper_accelerator, set_whisper_gpu_device, OrtAccelerator,
    WhisperAccelerator,
};
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

const WHISPER_SAMPLE_RATE: u32 = 16000;

#[derive(Parser, Debug)]
#[command(name = "whisper-benchmark")]
#[command(about = "Benchmark Whisper transcription with various settings")]
struct Args {
    /// Path to Whisper model GGUF file
    #[arg(long)]
    model: PathBuf,

    /// Path to audio file (WAV)
    #[arg(long, default_value = "src-tauri/handy-1775677205.wav")]
    audio: PathBuf,

    /// Whisper accelerator: auto, cpu, gpu
    #[arg(long, default_value = "auto")]
    whisper_accel: WhisperArg,

    /// ORT accelerator: auto, cpu, cuda, directml, rocm
    #[arg(long, default_value = "auto")]
    ort_accel: OrtArg,

    /// GPU device ID (-1 for auto)
    #[arg(long, default_value_t = -1)]
    gpu_device: i32,

    /// Language for transcription (auto for language detection)
    #[arg(long, default_value = "auto")]
    language: String,

    /// Translate to English
    #[arg(long)]
    translate: bool,

    /// Initial prompt / custom words
    #[arg(long)]
    prompt: Option<String>,

    /// Number of iterations to run
    #[arg(long, default_value_t = 3)]
    iterations: i32,

    /// Run warmup iteration first
    #[arg(long)]
    warmup: bool,
}

#[derive(Debug, Clone, ValueEnum)]
enum WhisperArg {
    Auto,
    Cpu,
    GPU,
}

#[derive(Debug, Clone, ValueEnum)]
enum OrtArg {
    Auto,
    Cpu,
    Cuda,
    DirectMl,
    Rocm,
}

fn load_audio(path: &PathBuf) -> Result<(Vec<f32>, u32)> {
    if !path.exists() {
        anyhow::bail!(
            "Audio file not found: {:?}\n\
             Use --audio to specify a different path.",
            path
        );
    }

    let mut reader = WavReader::open(path).context("Failed to open audio file")?;

    let spec: WavSpec = reader.spec().clone();
    let src_sample_rate = spec.sample_rate;
    let channels = spec.channels;

    println!("Audio file: {} channels, {} Hz", channels, src_sample_rate);

    let samples: Vec<f32> = match spec.sample_format {
        SampleFormat::Float => reader.samples::<f32>().filter_map(|s| s.ok()).collect(),
        SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
            reader
                .samples::<i32>()
                .map(|s| s.map(|v| v as f32 / max_val))
                .filter_map(|s| s.ok())
                .collect()
        }
    };

    // Convert stereo to mono
    let mono_audio: Vec<f32> = if channels == 2 {
        samples
            .chunks(2)
            .map(|chunk| (chunk[0] + chunk[1]) / 2.0)
            .collect()
    } else {
        samples
    };

    // Resample to 16kHz to match Handy app path
    let audio_16k = if src_sample_rate != WHISPER_SAMPLE_RATE {
        println!(
            "Resampling from {} Hz to {} Hz...",
            src_sample_rate, WHISPER_SAMPLE_RATE
        );
        resample_to_16k(&mono_audio, src_sample_rate as usize)?
    } else {
        mono_audio
    };

    let duration_sec = audio_16k.len() as f64 / WHISPER_SAMPLE_RATE as f64;
    println!(
        "Preprocessed audio: {:.2}s ({} samples, {} Hz mono)",
        duration_sec,
        audio_16k.len(),
        WHISPER_SAMPLE_RATE
    );

    Ok((audio_16k, WHISPER_SAMPLE_RATE))
}

fn resample_to_16k(audio: &[f32], src_rate: usize) -> Result<Vec<f32>> {
    const CHUNK_SIZE: usize = 1024;
    const OUT_RATE: usize = WHISPER_SAMPLE_RATE as usize;

    let mut resampler = FftFixedIn::<f32>::new(src_rate, OUT_RATE, CHUNK_SIZE, 1, 1)
        .context("Failed to create resampler")?;

    let mut result = Vec::new();

    // Process in chunks
    let mut input_buffer = Vec::with_capacity(CHUNK_SIZE);
    for chunk in audio.chunks(CHUNK_SIZE) {
        input_buffer.extend_from_slice(chunk);

        if input_buffer.len() == CHUNK_SIZE {
            let output = resampler
                .process(&[&input_buffer], None)
                .context("Resampling failed")?;
            result.extend_from_slice(&output[0]);
            input_buffer.clear();
        }
    }

    // Handle remaining samples
    if !input_buffer.is_empty() {
        input_buffer.resize(CHUNK_SIZE, 0.0);
        let output = resampler
            .process(&[&input_buffer], None)
            .context("Resampling failed")?;
        result.extend_from_slice(&output[0]);
    }

    Ok(result)
}

fn validate_model_path(path: &PathBuf) -> Result<()> {
    if !path.exists() {
        anyhow::bail!(
            "Model file not found: {:?}\n\
             Provide a valid path to a Whisper GGUF model.",
            path
        );
    }
    Ok(())
}

fn apply_accelerator_settings(whisper_accel: &WhisperArg, ort_accel: &OrtArg, gpu_device: i32) {
    let whisper = match whisper_accel {
        WhisperArg::Auto => WhisperAccelerator::Auto,
        WhisperArg::Cpu => WhisperAccelerator::CpuOnly,
        WhisperArg::GPU => WhisperAccelerator::Gpu,
    };
    set_whisper_accelerator(whisper);
    set_whisper_gpu_device(gpu_device);
    println!(
        "Whisper accelerator: {:?}, gpu_device: {}",
        whisper,
        if gpu_device == -1 {
            "auto".to_string()
        } else {
            gpu_device.to_string()
        }
    );

    let ort = match ort_accel {
        OrtArg::Auto => OrtAccelerator::Auto,
        OrtArg::Cpu => OrtAccelerator::CpuOnly,
        OrtArg::Cuda => OrtAccelerator::Cuda,
        OrtArg::DirectMl => OrtAccelerator::DirectMl,
        OrtArg::Rocm => OrtAccelerator::Rocm,
    };
    set_ort_accelerator(ort);
    println!("ORT accelerator: {:?}", ort);
}

fn main() -> Result<()> {
    let args = Args::parse();

    println!("=== Whisper Benchmark Tool ===");
    println!("Model: {:?}", args.model);
    println!("Audio: {:?}", args.audio);
    println!("");

    // Validate paths
    validate_model_path(&args.model)?;

    // Load and preprocess audio (resample to 16kHz mono)
    let (audio_16k, sample_rate) = load_audio(&args.audio)?;
    let audio_duration_sec = audio_16k.len() as f64 / sample_rate as f64;

    // Apply accelerator settings
    apply_accelerator_settings(&args.whisper_accel, &args.ort_accel, args.gpu_device);
    println!("");

    // Load model
    println!("Loading model...");
    let load_start = Instant::now();
    let mut engine = WhisperEngine::load(&args.model)?;
    let load_time = load_start.elapsed();
    println!("Model loaded in {:.2}s", load_time.as_secs_f64());
    println!("");

    // Build inference params
    let language = if args.language == "auto" {
        None
    } else {
        Some(args.language.clone())
    };

    let params = WhisperInferenceParams {
        language,
        translate: args.translate,
        initial_prompt: args.prompt.clone(),
        ..Default::default()
    };

    println!("Running transcription with settings:");
    println!("  language: {}", args.language);
    println!("  translate: {}", args.translate);
    if let Some(ref p) = args.prompt {
        println!("  prompt: {}", p);
    }
    println!("");

    // Warmup run
    if args.warmup {
        println!("--- Warmup run ---");
        let _ = engine.transcribe_with(&audio_16k, &params)?;
        println!("Warmup complete\n");
    }

    // Benchmark runs
    let mut run_times_ms: Vec<u64> = Vec::new();
    let mut transcription_result = String::new();

    for i in 1..=args.iterations {
        println!("--- Run {}/{} ---", i, args.iterations);
        let start = Instant::now();
        let result = engine.transcribe_with(&audio_16k, &params)?;
        let elapsed = start.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        run_times_ms.push(elapsed_ms);
        println!("Time: {}ms", elapsed_ms);

        // Print excerpt of transcription
        let text = &result.text;
        let excerpt = if text.len() > 100 {
            format!("{}...", &text[..100])
        } else {
            text.clone()
        };
        println!("Text: {}", excerpt);
        println!("");

        // Keep first non-empty result
        if transcription_result.is_empty() && !result.text.is_empty() {
            transcription_result = result.text;
        }
    }

    // Statistics
    let avg_ms = run_times_ms.iter().sum::<u64>() as f64 / run_times_ms.len() as f64;
    let min_ms = *run_times_ms.iter().min().unwrap();
    let max_ms = *run_times_ms.iter().max().unwrap();

    let rtf = audio_duration_sec / (avg_ms as f64 / 1000.0);

    println!("=== Results ===");
    println!("Audio duration (16kHz mono): {:.2}s", audio_duration_sec);
    println!("Sample rate: {} Hz", sample_rate);
    println!("Iterations: {}", args.iterations);
    println!("");
    println!("Timing (ms):");
    println!("  Min:  {}", min_ms);
    println!("  Max:  {}", max_ms);
    println!("  Avg:  {:.1}", avg_ms);
    println!("");
    println!("Real-time factor: {:.2}x (lower is faster)", rtf);
    println!("");

    if !transcription_result.is_empty() {
        println!("=== Full Transcription ===");
        println!("{}", transcription_result);
    }

    Ok(())
}
