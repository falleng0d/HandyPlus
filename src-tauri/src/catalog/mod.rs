use once_cell::sync::Lazy;
use serde::Deserialize;
use std::collections::HashMap;

use crate::managers::model::{EngineType, ModelInfo};

#[derive(Deserialize)]
struct Catalog {
    language_sets: HashMap<String, Vec<String>>,
    models: Vec<CatalogModel>,
}

#[derive(Deserialize)]
struct CatalogModel {
    id: String,
    name: String,
    description: String,
    #[serde(default)]
    languages: Vec<String>,
    language_set: Option<String>,
    capabilities: Capabilities,
    speed_score: f32,
    accuracy_score: f32,
    file: CatalogFile,
    recommended: bool,
}

#[derive(Deserialize)]
struct Capabilities {
    streaming: bool,
    translate: bool,
}

#[derive(Deserialize)]
struct CatalogFile {
    filename: String,
    size_bytes: u64,
}

static MODELS: Lazy<Vec<ModelInfo>> = Lazy::new(|| {
    let catalog: Catalog = serde_json::from_str(include_str!("catalog.json"))
        .expect("bundled model catalog must be valid");

    let language_sets = catalog.language_sets;
    catalog
        .models
        .into_iter()
        .filter(|model| !model.capabilities.streaming)
        .map(|model| {
            let filename = model.file.filename;
            let id = format!("{}/{}", model.id, filename);
            let languages = model
                .language_set
                .as_ref()
                .and_then(|name| language_sets.get(name))
                .cloned()
                .unwrap_or(model.languages);
            ModelInfo {
                id,
                name: model.name,
                description: model.description,
                url: Some(format!(
                    "https://huggingface.co/{}/resolve/main/{}",
                    model.id, filename
                )),
                filename,
                sha256: None,
                size_mb: model.file.size_bytes / 1_048_576,
                is_downloaded: false,
                is_downloading: false,
                partial_size: 0,
                is_directory: false,
                engine_type: EngineType::Whisper,
                accuracy_score: model.accuracy_score / 100.0,
                speed_score: model.speed_score / 100.0,
                supports_translation: model.capabilities.translate,
                is_recommended: model.recommended,
                supports_language_selection: languages.len() > 1,
                supported_languages: languages,
                is_custom: false,
            }
        })
        .collect()
});

pub fn models() -> Vec<ModelInfo> {
    MODELS.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn catalog_contains_exactly_the_non_streaming_upstream_models() {
        let raw: Catalog = serde_json::from_str(include_str!("catalog.json")).unwrap();
        assert_eq!(raw.models.len(), 60);
        assert!(raw.models.iter().all(|model| !model.capabilities.streaming));

        let models = models();
        assert_eq!(models.len(), 60);
        assert_eq!(
            models
                .iter()
                .map(|model| model.id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            60
        );
        assert!(models.iter().all(|model| {
            model.filename.ends_with(".gguf")
                && model.url.as_deref().is_some_and(|url| {
                    url.starts_with("https://huggingface.co/") && url.ends_with(&model.filename)
                })
        }));
    }
}
