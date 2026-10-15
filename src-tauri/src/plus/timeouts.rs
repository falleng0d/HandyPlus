use crate::managers::transcription::TranscriptionManager;
use crate::settings::AppSettings;
use log::warn;
use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

/// Runs upstream's finalize-or-transcribe step on a blocking thread, giving up
/// after `transcription_timeout_secs` (0 waits forever). A timed-out engine
/// call keeps running in the background; the recording stays in history so
/// it can be retried.
pub async fn transcribe(
    settings: &AppSettings,
    tm: Arc<TranscriptionManager>,
    samples: Vec<f32>,
) -> anyhow::Result<String> {
    let work = tokio::task::spawn_blocking(move || match tm.finalize_stream() {
        Ok(Some(text)) if !text.trim().is_empty() => Ok(text),
        Ok(_) => tm.transcribe(samples),
        Err(err) => Err(err),
    });
    let joined = match settings.transcription_timeout_secs {
        0 => work.await,
        secs => match tokio::time::timeout(Duration::from_secs(secs.into()), work).await {
            Ok(joined) => joined,
            Err(_) => {
                warn!("Transcription timed out after {secs}s");
                return Err(anyhow::anyhow!(
                    "Transcription timed out after {secs} seconds"
                ));
            }
        },
    };
    joined.map_err(|e| anyhow::anyhow!("Transcription task failed: {e}"))?
}

/// Gives post-processing `post_process_timeout_secs` (0 waits forever); on
/// timeout the caller pastes the unprocessed transcript.
pub async fn post_process<F>(settings: &AppSettings, work: F) -> Option<String>
where
    F: Future<Output = Option<String>>,
{
    match settings.post_process_timeout_secs {
        0 => work.await,
        secs => tokio::time::timeout(Duration::from_secs(secs.into()), work)
            .await
            .unwrap_or_else(|_| {
                warn!("Post-processing timed out after {secs}s; using the raw transcript");
                None
            }),
    }
}
