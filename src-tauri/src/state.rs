use std::sync::{Arc, Mutex};

/// Tracks the last successfully produced transcript for clipboard access.
#[derive(Clone)]
pub struct LastTranscriptState {
    inner: Arc<Mutex<Option<String>>>,
}

impl LastTranscriptState {
    /// Create a new empty state.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(None)),
        }
    }

    /// Replace the stored transcript.
    pub fn set(&self, transcript: Option<String>) {
        let mut guard = self.inner.lock().unwrap();
        *guard = transcript;
    }

    /// Return the currently stored transcript, if any.
    pub fn get(&self) -> Option<String> {
        self.inner.lock().unwrap().clone()
    }

    /// Returns true if a non-empty transcript is stored.
    pub fn has_transcript(&self) -> bool {
        self.inner
            .lock()
            .unwrap()
            .as_ref()
            .map(|text| !text.is_empty())
            .unwrap_or(false)
    }
}
