use crate::audio_toolkit::{apply_custom_words, filter_transcription_output};
use crate::managers::audio::AudioRecordingManager;
use crate::managers::model::{EngineType, ModelManager};
use crate::settings::{
    get_settings, ModelUnloadTimeout, OrtAcceleratorSetting, WhisperAcceleratorSetting,
};
use anyhow::Result;
use log::{debug, error, info, warn};
use serde::Serialize;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime};
use tauri::{AppHandle, Emitter, Manager};
use transcribe_cpp::{
    Backend, Feature, Model, ModelOptions, RunExtension, RunOptions, Session, Task,
    WhisperRunOptions,
};
use transcribe_rs::{
    onnx::{
        canary::CanaryModel,
        cohere::CohereModel,
        gigaam::GigaAMModel,
        moonshine::{MoonshineModel, MoonshineVariant},
        parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity},
        sense_voice::{SenseVoiceModel, SenseVoiceParams},
        Quantization,
    },
    SpeechModel, TranscribeOptions,
};

#[derive(Clone, Debug, Serialize)]
pub struct ModelStateEvent {
    pub event_type: String,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub error: Option<String>,
}

enum LoadedEngine {
    TranscribeCpp(Session),
    Parakeet(ParakeetModel),
    Moonshine(MoonshineModel),
    SenseVoice(SenseVoiceModel),
    GigaAM(GigaAMModel),
    Canary(CanaryModel),
    Cohere(CohereModel),
}

struct LoadingGuard {
    is_loading: Arc<Mutex<bool>>,
    loading_condvar: Arc<Condvar>,
}

impl Drop for LoadingGuard {
    fn drop(&mut self) {
        let mut is_loading = self.is_loading.lock().unwrap();
        *is_loading = false;
        self.loading_condvar.notify_all();
    }
}

#[derive(Clone)]
pub struct TranscriptionManager {
    engine: Arc<Mutex<Option<LoadedEngine>>>,
    model_manager: Arc<ModelManager>,
    app_handle: AppHandle,
    current_model_id: Arc<Mutex<Option<String>>>,
    last_activity: Arc<AtomicU64>,
    shutdown_signal: Arc<AtomicBool>,
    watcher_handle: Arc<Mutex<Option<thread::JoinHandle<()>>>>,
    is_loading: Arc<Mutex<bool>>,
    loading_condvar: Arc<Condvar>,
}

impl TranscriptionManager {
    pub fn new(app_handle: &AppHandle, model_manager: Arc<ModelManager>) -> Result<Self> {
        let manager = Self {
            engine: Arc::new(Mutex::new(None)),
            model_manager,
            app_handle: app_handle.clone(),
            current_model_id: Arc::new(Mutex::new(None)),
            last_activity: Arc::new(AtomicU64::new(Self::now_ms())),
            shutdown_signal: Arc::new(AtomicBool::new(false)),
            watcher_handle: Arc::new(Mutex::new(None)),
            is_loading: Arc::new(Mutex::new(false)),
            loading_condvar: Arc::new(Condvar::new()),
        };

        // Start the idle watcher
        {
            let app_handle_cloned = app_handle.clone();
            let manager_cloned = manager.clone();
            let shutdown_signal = manager.shutdown_signal.clone();
            let handle = thread::spawn(move || {
                debug!("Idle watcher thread started");
                while !shutdown_signal.load(Ordering::Relaxed) {
                    thread::sleep(Duration::from_secs(10)); // Check every 10 seconds

                    // Check shutdown signal again after sleep
                    if shutdown_signal.load(Ordering::Relaxed) {
                        break;
                    }

                    let settings = get_settings(&app_handle_cloned);
                    let timeout = settings.model_unload_timeout;

                    if timeout == ModelUnloadTimeout::Immediately {
                        continue;
                    }

                    let is_recording = app_handle_cloned
                        .try_state::<Arc<AudioRecordingManager>>()
                        .map_or(false, |audio_manager| audio_manager.is_recording());
                    if is_recording {
                        manager_cloned.touch_activity();
                        continue;
                    }

                    if let Some(limit_seconds) = timeout.to_seconds() {
                        let last = manager_cloned.last_activity.load(Ordering::Relaxed);
                        let now_ms = Self::now_ms();

                        if now_ms.saturating_sub(last) > limit_seconds * 1000 {
                            if manager_cloned.is_model_loaded() {
                                let unload_start = std::time::Instant::now();
                                info!(
                                    "Model idle for {}s (limit: {}s), unloading",
                                    now_ms.saturating_sub(last) / 1000,
                                    limit_seconds
                                );

                                match manager_cloned.unload_model() {
                                    Ok(()) => {
                                        let unload_duration = unload_start.elapsed();
                                        info!(
                                            "Model unloaded due to inactivity (took {}ms)",
                                            unload_duration.as_millis()
                                        );
                                    }
                                    Err(e) => error!("Failed to unload idle model: {}", e),
                                }
                            }
                        }
                    }
                }
                debug!("Idle watcher thread shutting down gracefully");
            });
            *manager.watcher_handle.lock().unwrap() = Some(handle);
        }

        Ok(manager)
    }

    fn lock_engine(&self) -> MutexGuard<'_, Option<LoadedEngine>> {
        self.engine.lock().unwrap_or_else(|poisoned| {
            warn!("Engine mutex was poisoned by a previous panic, recovering");
            poisoned.into_inner()
        })
    }

    fn now_ms() -> u64 {
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn touch_activity(&self) {
        self.last_activity.store(Self::now_ms(), Ordering::Relaxed);
    }

    fn try_start_loading(&self) -> Option<LoadingGuard> {
        let mut is_loading = self.is_loading.lock().unwrap();
        if *is_loading {
            return None;
        }
        *is_loading = true;
        Some(LoadingGuard {
            is_loading: self.is_loading.clone(),
            loading_condvar: self.loading_condvar.clone(),
        })
    }

    pub fn is_model_loaded(&self) -> bool {
        let engine = self.lock_engine();
        engine.is_some()
    }

    pub fn is_loading(&self) -> bool {
        *self.is_loading.lock().unwrap()
    }

    pub fn unload_model(&self) -> Result<()> {
        let unload_start = std::time::Instant::now();
        debug!("Starting to unload model");

        {
            let mut engine = self.lock_engine();
            *engine = None;
        }
        {
            let mut current_model = self.current_model_id.lock().unwrap();
            *current_model = None;
        }

        // Emit unloaded event
        let _ = self.app_handle.emit(
            "model-state-changed",
            ModelStateEvent {
                event_type: "unloaded".to_string(),
                model_id: None,
                model_name: None,
                error: None,
            },
        );

        let unload_duration = unload_start.elapsed();
        debug!(
            "Model unloaded manually (took {}ms)",
            unload_duration.as_millis()
        );
        Ok(())
    }

    /// Unloads the model immediately if the setting is enabled and the model is loaded
    pub fn maybe_unload_immediately(&self, context: &str) {
        let settings = get_settings(&self.app_handle);
        if settings.model_unload_timeout == ModelUnloadTimeout::Immediately
            && self.is_model_loaded()
        {
            info!("Immediately unloading model after {}", context);
            if let Err(e) = self.unload_model() {
                warn!("Failed to immediately unload model: {}", e);
            }
        }
    }

    pub fn load_model(&self, model_id: &str) -> Result<()> {
        let load_start = std::time::Instant::now();
        debug!("Starting to load model: {}", model_id);

        // Emit loading started event
        let _ = self.app_handle.emit(
            "model-state-changed",
            ModelStateEvent {
                event_type: "loading_started".to_string(),
                model_id: Some(model_id.to_string()),
                model_name: None,
                error: None,
            },
        );

        let model_info = self
            .model_manager
            .get_model_info(model_id)
            .ok_or_else(|| anyhow::anyhow!("Model not found: {}", model_id))?;

        if !model_info.is_downloaded {
            let error_msg = "Model not downloaded";
            let _ = self.app_handle.emit(
                "model-state-changed",
                ModelStateEvent {
                    event_type: "loading_failed".to_string(),
                    model_id: Some(model_id.to_string()),
                    model_name: Some(model_info.name.clone()),
                    error: Some(error_msg.to_string()),
                },
            );
            return Err(anyhow::anyhow!(error_msg));
        }

        let model_path = self.model_manager.get_model_path(model_id)?;

        let emit_loading_failed = |error_msg: &str| {
            let _ = self.app_handle.emit(
                "model-state-changed",
                ModelStateEvent {
                    event_type: "loading_failed".to_string(),
                    model_id: Some(model_id.to_string()),
                    model_name: Some(model_info.name.clone()),
                    error: Some(error_msg.to_string()),
                },
            );
        };

        let loaded_engine = match model_info.engine_type {
            EngineType::Whisper => {
                let settings = get_settings(&self.app_handle);
                let backend = select_transcribe_backend(settings.whisper_accelerator);
                let gpu_device =
                    resolve_gpu_device(settings.whisper_accelerator, settings.whisper_gpu_device);
                let model = Model::load_with(
                    &model_path,
                    &ModelOptions {
                        backend,
                        gpu_device,
                    },
                )
                .map_err(|e| {
                    let error_msg = format!("Failed to load whisper model {}: {}", model_id, e);
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                let bound_backend = model.backend();
                let session = model.session().map_err(|e| {
                    let error_msg = format!(
                        "Failed to create transcription session for {}: {}",
                        model_id, e
                    );
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                info!(
                    "Loaded transcribe-cpp model '{}' (requested {:?}, gpu_device {}, bound backend '{}')",
                    model_id, backend, gpu_device, bound_backend
                );
                LoadedEngine::TranscribeCpp(session)
            }
            EngineType::Parakeet => {
                let engine =
                    ParakeetModel::load(&model_path, &Quantization::Int8).map_err(|e| {
                        let error_msg =
                            format!("Failed to load parakeet model {}: {}", model_id, e);
                        emit_loading_failed(&error_msg);
                        anyhow::anyhow!(error_msg)
                    })?;
                LoadedEngine::Parakeet(engine)
            }
            EngineType::Moonshine => {
                let engine = MoonshineModel::load(
                    &model_path,
                    MoonshineVariant::Base,
                    &Quantization::default(),
                )
                .map_err(|e| {
                    let error_msg = format!("Failed to load moonshine model {}: {}", model_id, e);
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                LoadedEngine::Moonshine(engine)
            }
            EngineType::SenseVoice => {
                let engine =
                    SenseVoiceModel::load(&model_path, &Quantization::Int8).map_err(|e| {
                        let error_msg =
                            format!("Failed to load sensevoice model {}: {}", model_id, e);
                        emit_loading_failed(&error_msg);
                        anyhow::anyhow!(error_msg)
                    })?;
                LoadedEngine::SenseVoice(engine)
            }
            EngineType::GigaAM => {
                let engine = GigaAMModel::load(&model_path, &Quantization::Int8).map_err(|e| {
                    let error_msg = format!("Failed to load gigaam model {}: {}", model_id, e);
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                LoadedEngine::GigaAM(engine)
            }
            EngineType::Canary => {
                let engine = CanaryModel::load(&model_path, &Quantization::Int8).map_err(|e| {
                    let error_msg = format!("Failed to load canary model {}: {}", model_id, e);
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                LoadedEngine::Canary(engine)
            }
            EngineType::Cohere => {
                let engine = CohereModel::load(&model_path, &Quantization::Int8).map_err(|e| {
                    let error_msg = format!("Failed to load cohere model {}: {}", model_id, e);
                    emit_loading_failed(&error_msg);
                    anyhow::anyhow!(error_msg)
                })?;
                LoadedEngine::Cohere(engine)
            }
        };

        // Update the current engine and model ID
        {
            let mut engine = self.engine.lock().unwrap();
            *engine = Some(loaded_engine);
        }
        {
            let mut current_model = self.current_model_id.lock().unwrap();
            *current_model = Some(model_id.to_string());
        }

        self.touch_activity();

        // Emit loading completed event
        let _ = self.app_handle.emit(
            "model-state-changed",
            ModelStateEvent {
                event_type: "loading_completed".to_string(),
                model_id: Some(model_id.to_string()),
                model_name: Some(model_info.name.clone()),
                error: None,
            },
        );

        let load_duration = load_start.elapsed();
        debug!(
            "Successfully loaded transcription model: {} (took {}ms)",
            model_id,
            load_duration.as_millis()
        );
        Ok(())
    }

    /// Kicks off the model loading in a background thread if it's not already loaded
    pub fn initiate_model_load(&self) {
        if self.is_model_loaded() {
            return;
        }

        let Some(_guard) = self.try_start_loading() else {
            return;
        };

        let self_clone = self.clone();
        thread::spawn(move || {
            let _guard = _guard;
            let settings = get_settings(&self_clone.app_handle);
            if let Err(e) = self_clone.load_model(&settings.selected_model) {
                error!("Failed to load model: {}", e);
            }
        });
    }

    pub fn get_current_model(&self) -> Option<String> {
        let current_model = self.current_model_id.lock().unwrap();
        current_model.clone()
    }

    pub fn transcribe(&self, audio: Vec<f32>) -> Result<String> {
        // Update last activity timestamp
        self.touch_activity();

        let st = std::time::Instant::now();

        debug!("Audio vector length: {}", audio.len());

        if audio.is_empty() {
            debug!("Empty audio vector");
            self.maybe_unload_immediately("empty audio");
            return Ok(String::new());
        }

        // Check if model is loaded, if not try to load it
        {
            // If the model is loading, wait for it to complete.
            let mut is_loading = self.is_loading.lock().unwrap();
            while *is_loading {
                is_loading = self.loading_condvar.wait(is_loading).unwrap();
            }

            let engine_guard = self.lock_engine();
            if engine_guard.is_none() {
                return Err(anyhow::anyhow!("Model is not loaded for transcription."));
            }
        }

        // Get current settings for configuration
        let settings = get_settings(&self.app_handle);

        let validated_language = if settings.selected_language == "auto" {
            "auto".to_string()
        } else {
            let is_supported = self
                .model_manager
                .get_model_info(&settings.selected_model)
                .map(|info| {
                    info.supported_languages.is_empty()
                        || info
                            .supported_languages
                            .contains(&settings.selected_language)
                })
                .unwrap_or(true);

            if is_supported {
                settings.selected_language.clone()
            } else {
                warn!(
                    "Language '{}' not supported by current model, falling back to auto-detect",
                    settings.selected_language
                );
                "auto".to_string()
            }
        };

        // Run engine inference without holding the mutex so a panic cannot poison it.
        let custom_words_already_prompted;
        let result = {
            let mut engine_guard = self.lock_engine();
            let mut engine = match engine_guard.take() {
                Some(engine) => engine,
                None => {
                    return Err(anyhow::anyhow!(
                        "Model failed to load after auto-load attempt. Please check your model settings."
                    ));
                }
            };
            drop(engine_guard);

            let (model_is_whisper, supports_initial_prompt, supports_translate, languages) =
                match &engine {
                    LoadedEngine::TranscribeCpp(session) => {
                        let model = session.model();
                        let caps = model.capabilities();
                        (
                            model.arch() == "whisper",
                            model.supports(Feature::InitialPrompt),
                            caps.supports_translate,
                            caps.languages,
                        )
                    }
                    _ => (false, false, false, Vec::new()),
                };
            custom_words_already_prompted = settings.custom_word_correction_enabled
                && !settings.custom_words.is_empty()
                && model_is_whisper
                && supports_initial_prompt;

            let transcribe_result = catch_unwind(AssertUnwindSafe(|| -> Result<_> {
                match &mut engine {
                    LoadedEngine::TranscribeCpp(session) => {
                        let language = if validated_language == "auto" {
                            None
                        } else {
                            let normalized = normalize_language(&validated_language).to_string();
                            languages.contains(&normalized).then_some(normalized)
                        };
                        let translate = settings.translate_to_english
                            && supports_translate
                            && language.as_deref() != Some("en");
                        let family = if custom_words_already_prompted {
                            Some(RunExtension::Whisper(WhisperRunOptions {
                                initial_prompt: Some(settings.custom_words.join(", ")),
                                ..Default::default()
                            }))
                        } else {
                            None
                        };
                        let options = RunOptions {
                            task: if translate {
                                Task::Translate
                            } else {
                                Task::Transcribe
                            },
                            language,
                            target_language: translate.then(|| "en".to_string()),
                            family,
                            ..Default::default()
                        };
                        session
                            .run(&audio, &options)
                            .map(|result| result.text)
                            .map_err(|e| {
                                anyhow::anyhow!("transcribe-cpp transcription failed: {}", e)
                            })
                    }
                    LoadedEngine::Parakeet(parakeet_engine) => {
                        let params = ParakeetParams {
                            timestamp_granularity: Some(TimestampGranularity::Segment),
                            ..Default::default()
                        };
                        parakeet_engine
                            .transcribe_with(&audio, &params)
                            .map(|result| result.text)
                            .map_err(|e| anyhow::anyhow!("Parakeet transcription failed: {}", e))
                    }
                    LoadedEngine::Moonshine(moonshine_engine) => moonshine_engine
                        .transcribe(&audio, &TranscribeOptions::default())
                        .map(|result| result.text)
                        .map_err(|e| anyhow::anyhow!("Moonshine transcription failed: {}", e)),
                    LoadedEngine::SenseVoice(sensevoice_engine) => {
                        let normalized = match validated_language.as_str() {
                            "zh-Hans" | "zh-Hant" => Some("zh".to_string()),
                            "en" => Some("en".to_string()),
                            "ja" => Some("ja".to_string()),
                            "ko" => Some("ko".to_string()),
                            "yue" => Some("yue".to_string()),
                            _ => None,
                        };

                        let params = SenseVoiceParams {
                            language: normalized,
                            use_itn: Some(true),
                        };

                        sensevoice_engine
                            .transcribe_with(&audio, &params)
                            .map(|result| result.text)
                            .map_err(|e| anyhow::anyhow!("SenseVoice transcription failed: {}", e))
                    }
                    LoadedEngine::GigaAM(gigaam_engine) => gigaam_engine
                        .transcribe(&audio, &TranscribeOptions::default())
                        .map(|result| result.text)
                        .map_err(|e| anyhow::anyhow!("GigaAM transcription failed: {}", e)),
                    LoadedEngine::Canary(canary_engine) => {
                        let language = if validated_language == "auto" {
                            None
                        } else if validated_language == "zh-Hans" || validated_language == "zh-Hant"
                        {
                            Some("zh".to_string())
                        } else {
                            Some(validated_language.clone())
                        };

                        let options = TranscribeOptions {
                            language,
                            translate: settings.translate_to_english,
                            ..Default::default()
                        };

                        canary_engine
                            .transcribe(&audio, &options)
                            .map(|result| result.text)
                            .map_err(|e| anyhow::anyhow!("Canary transcription failed: {}", e))
                    }
                    LoadedEngine::Cohere(cohere_engine) => {
                        let language = if validated_language == "auto" {
                            None
                        } else if validated_language == "zh-Hans" || validated_language == "zh-Hant"
                        {
                            Some("zh".to_string())
                        } else {
                            Some(validated_language.clone())
                        };

                        let options = TranscribeOptions {
                            language,
                            ..Default::default()
                        };

                        cohere_engine
                            .transcribe(&audio, &options)
                            .map(|result| result.text)
                            .map_err(|e| anyhow::anyhow!("Cohere transcription failed: {}", e))
                    }
                }
            }));

            match transcribe_result {
                Ok(inner_result) => {
                    let mut engine_guard = self.lock_engine();
                    *engine_guard = Some(engine);
                    inner_result?
                }
                Err(panic_payload) => {
                    let panic_msg = if let Some(message) = panic_payload.downcast_ref::<&str>() {
                        (*message).to_string()
                    } else if let Some(message) = panic_payload.downcast_ref::<String>() {
                        message.clone()
                    } else {
                        "unknown panic".to_string()
                    };

                    error!(
                        "Transcription engine panicked: {}. Model has been unloaded.",
                        panic_msg
                    );

                    {
                        let mut current_model = self.current_model_id.lock().unwrap();
                        *current_model = None;
                    }

                    let _ = self.app_handle.emit(
                        "model-state-changed",
                        ModelStateEvent {
                            event_type: "unloaded".to_string(),
                            model_id: None,
                            model_name: None,
                            error: Some(format!("Engine panicked: {}", panic_msg)),
                        },
                    );

                    return Err(anyhow::anyhow!(
                        "Transcription engine panicked: {}. The model has been unloaded and will reload on next attempt.",
                        panic_msg
                    ));
                }
            }
        };

        let corrected_result = if settings.custom_word_correction_enabled
            && !settings.custom_words.is_empty()
            && !custom_words_already_prompted
        {
            apply_custom_words(
                &result,
                &settings.custom_words,
                settings.word_correction_threshold,
            )
        } else {
            result
        };

        // Filter out filler words and hallucinations
        let filtered_result = filter_transcription_output(&corrected_result);

        let et = std::time::Instant::now();
        let translation_note = if settings.translate_to_english {
            " (translated)"
        } else {
            ""
        };
        info!(
            "Transcription completed in {}ms{}",
            (et - st).as_millis(),
            translation_note
        );

        let final_result = filtered_result;

        if final_result.is_empty() {
            info!("Transcription result is empty");
        } else {
            info!("Transcription result: {}", final_result);
        }

        self.maybe_unload_immediately("transcription");

        Ok(final_result)
    }
}

fn normalize_language(language: &str) -> &str {
    match language {
        "zh-Hans" | "zh-Hant" => "zh",
        other => other,
    }
}

/// Initialize transcribe-cpp logging and dynamic compute backends once, before
/// any model is loaded.
pub fn init_transcribe_backend() {
    transcribe_cpp::init_logging();
    match transcribe_cpp::init_backends_default() {
        Ok(()) => info!(
            "transcribe-cpp initialized with {} compute device(s)",
            transcribe_cpp::devices().len()
        ),
        Err(error) => warn!("Failed to initialize transcribe-cpp backends: {}", error),
    }
}

fn select_transcribe_backend(setting: WhisperAcceleratorSetting) -> Backend {
    match setting {
        WhisperAcceleratorSetting::Auto => Backend::Auto,
        WhisperAcceleratorSetting::Cpu => Backend::Cpu,
        WhisperAcceleratorSetting::Gpu => {
            #[cfg(target_os = "macos")]
            let candidates = [Backend::Metal];
            #[cfg(not(target_os = "macos"))]
            let candidates = [Backend::Cuda, Backend::Vulkan];

            candidates
                .into_iter()
                .find(|backend| transcribe_cpp::backend_available(*backend))
                .unwrap_or_else(|| {
                    warn!("No transcribe-cpp GPU backend available; falling back to Auto");
                    Backend::Auto
                })
        }
    }
}

fn is_gpu_device(device: &transcribe_cpp::Device) -> bool {
    device.kind != "cpu" && device.kind != "accel"
}

fn resolve_gpu_device(setting: WhisperAcceleratorSetting, gpu_device: i32) -> i32 {
    if setting != WhisperAcceleratorSetting::Gpu || gpu_device <= 0 {
        return 0;
    }
    if transcribe_cpp::devices()
        .iter()
        .any(|device| device.index == Some(gpu_device as usize) && is_gpu_device(device))
    {
        gpu_device
    } else {
        warn!(
            "Stored transcribe-cpp GPU device {} is unavailable; using auto",
            gpu_device
        );
        0
    }
}

/// Apply the configured ONNX Runtime accelerator before any model is loaded.
pub fn apply_accelerator_settings(app: &tauri::AppHandle) {
    use transcribe_rs::accel;

    let settings = get_settings(app);
    info!(
        "accelerator preferences: whisper={:?}, ort={:?}, gpu_device={} (applied on model load)",
        settings.whisper_accelerator, settings.ort_accelerator, settings.whisper_gpu_device
    );
    let ort_accelerator = match settings.ort_accelerator {
        OrtAcceleratorSetting::Auto => accel::OrtAccelerator::Auto,
        OrtAcceleratorSetting::Cpu => accel::OrtAccelerator::CpuOnly,
        OrtAcceleratorSetting::Cuda => accel::OrtAccelerator::Cuda,
        OrtAcceleratorSetting::DirectMl => accel::OrtAccelerator::DirectMl,
        OrtAcceleratorSetting::Rocm => accel::OrtAccelerator::Rocm,
    };
    accel::set_ort_accelerator(ort_accelerator);
}

#[derive(Serialize, Clone, Debug)]
pub struct GpuDeviceOption {
    pub id: i32,
    pub name: String,
    pub total_vram_mb: usize,
}

static GPU_DEVICES: OnceLock<Vec<GpuDeviceOption>> = OnceLock::new();

fn cached_gpu_devices() -> &'static [GpuDeviceOption] {
    GPU_DEVICES.get_or_init(|| {
        transcribe_cpp::devices()
            .into_iter()
            .filter(is_gpu_device)
            .map(|d| GpuDeviceOption {
                id: d.index.unwrap_or(0) as i32,
                name: if d.description.is_empty() {
                    d.name
                } else {
                    d.description
                },
                total_vram_mb: (d.memory_total / (1024 * 1024)) as usize,
            })
            .collect()
    })
}

#[derive(Serialize, Clone, Debug)]
pub struct AvailableAccelerators {
    pub whisper: Vec<String>,
    pub ort: Vec<String>,
    pub gpu_devices: Vec<GpuDeviceOption>,
}

/// Return which accelerators are compiled into this build.
pub fn get_available_accelerators() -> AvailableAccelerators {
    let whisper_options = vec!["auto".to_string(), "cpu".to_string(), "gpu".to_string()];

    AvailableAccelerators {
        whisper: whisper_options,
        ort: vec!["auto".to_string(), "cpu".to_string(), "cuda".to_string()],
        gpu_devices: cached_gpu_devices().to_vec(),
    }
}

impl Drop for TranscriptionManager {
    fn drop(&mut self) {
        // Skip shutdown unless this is the very last clone. TranscriptionManager
        // is cloned by initiate_model_load() and the watcher thread — those
        // clones dropping must not kill the watcher. The watcher thread holds
        // its own clone, so engine's strong_count is always >= 2 while the
        // watcher is alive. When it reaches 1, only this instance remains
        // and we can safely shut down.
        if Arc::strong_count(&self.engine) > 1 {
            return;
        }

        debug!("Shutting down TranscriptionManager");

        // Signal the watcher thread to shutdown
        self.shutdown_signal.store(true, Ordering::Relaxed);

        // Wait for the thread to finish gracefully
        if let Some(handle) = self.watcher_handle.lock().unwrap().take() {
            if let Err(e) = handle.join() {
                warn!("Failed to join idle watcher thread: {:?}", e);
            } else {
                debug!("Idle watcher thread joined successfully");
            }
        }
    }
}
