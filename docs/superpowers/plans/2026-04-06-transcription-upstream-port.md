# Transcription Upstream Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring the upstream transcription engines, new model catalog, accelerator tuning, and the minimum supporting UI/settings into this branch without pulling in unrelated upstream changes.

**Architecture:** Keep this branch's current transcription manager structure, concurrency behavior, panic isolation, and unload logic. Port only the missing engine/model/accelerator functionality from `upstream/main`, adapting it into the current codebase instead of replacing large files wholesale. Execute each epic on a dedicated task branch created from `transcription-upstream-port`, then merge back into the base branch after review.

**Tech Stack:** Rust, Tauri, transcribe-rs, React, TypeScript, Zustand, Bun

---

## File Map

**Core backend files**

- `src-tauri/src/managers/model.rs`: engine enum, model metadata, model migrations, download metadata
- `src-tauri/src/managers/transcription.rs`: engine loading, inference dispatch, accelerator helpers, idle/unload behavior
- `src-tauri/src/settings.rs`: persisted settings schema for accelerators and related tuning
- `src-tauri/src/lib.rs`: startup initialization and command registration
- `src-tauri/Cargo.toml`: backend dependency/features for engine support
- `src-tauri/Cargo.lock`: lockfile updates after dependency changes

**Supporting frontend files**

- `src/lib/types.ts`: Zod schemas and TS types for engine enums and settings
- `src/stores/settingsStore.ts`: invoke-based settings updates and default values
- `src/contexts/ModelsContext.tsx`: model loading/listing validation for restored engines/models
- `src/components/settings/`: location for accelerator controls if a new scoped UI component is needed

**Out of scope unless required by an epic**

- Post-processing provider changes unrelated to transcription engine work
- General settings refactors from upstream
- Translation/i18n updates not required for engine/model/accelerator support
- Unrelated tray/audio/history/UI work

## Branch Workflow

- [ ] Create and stay on base branch `transcription-upstream-port`
- [ ] For each epic, create a task branch from the current base branch using the format `task/<epic-name>`
- [ ] Instruct the implementer subagent to commit as it goes on its task branch
- [ ] After implementation, review the task branch for scope, correctness, and quality
- [ ] Merge the task branch back into `transcription-upstream-port`
- [ ] Delete the task branch after a successful merge

## Master Tasks

### Epic 1: Restore New Models And Engine Catalog

**Outcome:** The branch can advertise and manage the upstream transcription models that are currently missing, without yet requiring full accelerator UI work.

**Files:**

- Modify: `src-tauri/src/managers/model.rs`
- Modify: `src/lib/types.ts`
- Verify: `src/contexts/ModelsContext.tsx`
- Verify: `src/components/onboarding/Onboarding.tsx`

**Subtasks:**

- [ ] Expand `EngineType` in `src-tauri/src/managers/model.rs` to include `MoonshineStreaming`, `SenseVoice`, `GigaAM`, and `Canary`
- [ ] Restore the missing upstream model definitions in `ModelManager::new()`:
  - `breeze-asr`
  - `moonshine-tiny-streaming-en`
  - `moonshine-small-streaming-en`
  - `moonshine-medium-streaming-en`
  - `sense-voice-int8`
  - `gigaam-v3-e2e-ctc`
  - `canary-180m-flash`
  - `canary-1b-v2`
- [ ] Restore any model-specific language lists needed by those models
- [ ] Restore the GigaAM migration helper if the current branch still needs compatibility with the upstream directory layout
- [ ] Decide whether to keep or skip custom Whisper discovery; default is skip unless the restored model work requires it
- [ ] Update `src/lib/types.ts` so the frontend schema accepts the expanded engine enum
- [ ] Validate that existing frontend model consumers do not assume only four engine types

**Acceptance criteria:**

- `get_available_models` returns the restored upstream model set with valid engine types
- Existing model list consumers can parse and render the returned metadata
- No unrelated settings/UI behavior changes are introduced in this epic

### Epic 2: Restore Engine Loading And Inference Support

**Outcome:** The restored models from Epic 1 can actually load and transcribe through this branch's transcription manager.

**Files:**

- Modify: `src-tauri/src/managers/transcription.rs`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/Cargo.lock`
- Verify: `src-tauri/src/managers/model.rs`

**Subtasks:**

- [ ] Reintroduce the missing `LoadedEngine` variants in `transcription.rs`
- [ ] Restore model loading branches for `MoonshineStreaming`, `SenseVoice`, `GigaAM`, and `Canary`
- [ ] Restore per-engine inference logic for those engines
- [ ] Preserve this branch's current panic handling, lock discipline, watcher behavior, and immediate unload logic while adding the new engine branches
- [ ] Preserve the current branch's Whisper `initial_prompt` behavior and current custom-words handling
- [ ] Reconcile per-engine language normalization with the current branch's validation/fallback behavior
- [ ] Update `src-tauri/Cargo.toml` only if the current `transcribe-rs` feature set is insufficient for the restored engines

**Acceptance criteria:**

- Restored model IDs can be loaded through `TranscriptionManager::load_model`
- `TranscriptionManager::transcribe` can dispatch to every supported engine variant
- Existing concurrency and unload protections remain intact

### Epic 3: Restore Accelerator Settings And Backend Tuning Hooks

**Outcome:** The branch regains backend accelerator configuration and GPU enumeration needed for performance tuning.

**Files:**

- Modify: `src-tauri/src/settings.rs`
- Modify: `src-tauri/src/managers/transcription.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/shortcut/mod.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/stores/settingsStore.ts`

**Subtasks:**

- [ ] Restore `WhisperAcceleratorSetting`, `OrtAcceleratorSetting`, and `whisper_gpu_device` in `settings.rs`
- [ ] Restore default values and persistence wiring for those settings
- [ ] Restore `apply_accelerator_settings()` in `transcription.rs`
- [ ] Restore `GpuDeviceOption`, `AvailableAccelerators`, and `get_available_accelerators()` in `transcription.rs`
- [ ] Restore startup application of accelerator settings in `src-tauri/src/lib.rs`
- [ ] Restore invoke handlers in `src-tauri/src/lib.rs` for accelerator updates and accelerator enumeration
- [ ] Restore backend command functions in `src-tauri/src/shortcut/mod.rs`:
  - `change_whisper_accelerator_setting`
  - `change_ort_accelerator_setting`
  - `change_whisper_gpu_device`
  - `get_available_accelerators`
- [ ] Extend frontend types and settings-store updater wiring for the restored accelerator settings

**Acceptance criteria:**

- Accelerator settings persist in the store and can be changed through invoke handlers
- Available GPU devices and accelerator choices can be queried from the frontend
- Startup applies the saved accelerator preferences

### Epic 4: Add Minimal Frontend Support For Accelerators And Restored Engines

**Outcome:** Users can configure restored accelerators from the existing settings UI, and the frontend remains compatible with the expanded engine/model catalog.

**Files:**

- Modify: `src/lib/types.ts`
- Modify: `src/stores/settingsStore.ts`
- Create or Modify: `src/components/settings/AccelerationSelector.tsx`
- Modify: existing settings composition files under `src/components/settings/`

**Subtasks:**

- [ ] Add or port a scoped `AccelerationSelector` component compatible with this branch's settings architecture
- [ ] Surface the restored accelerator settings in the settings UI without pulling in unrelated upstream settings panels
- [ ] Ensure the selector supports whisper accelerator mode, ORT accelerator mode, and GPU device selection
- [ ] Ensure the current model/model list flows continue to work with the expanded engine enum from Epic 1
- [ ] Keep the UI intentionally minimal and limited to transcription-performance controls

**Acceptance criteria:**

- Users can select accelerator settings from the app UI
- The UI can parse and display expanded engine/model metadata without schema errors
- No unrelated settings screens are changed beyond what is required to expose the restored functionality

### Epic 5: Verification And Integration Cleanup

**Outcome:** The port is validated end-to-end and cleaned up for handoff.

**Files:**

- Verify all files touched by Epics 1-4

**Subtasks:**

- [ ] Run `bun check`
- [ ] Review model and accelerator code paths for dead references to removed settings or stale enum assumptions
- [ ] Confirm there are no missing invoke handlers or schema mismatches between Rust and TypeScript
- [ ] Confirm branch history is clean and each epic merged cleanly into `transcription-upstream-port`

**Acceptance criteria:**

- `bun check` passes
- The final branch contains only transcription/model/accelerator-focused changes
- No orphan task branches remain

## Execution Order

- [ ] Complete Epic 1 and merge it into `transcription-upstream-port`
- [ ] Complete Epic 2 and merge it into `transcription-upstream-port`
- [ ] Complete Epic 3 and merge it into `transcription-upstream-port`
- [ ] Complete Epic 4 and merge it into `transcription-upstream-port`
- [ ] Complete Epic 5 verification on the base branch

## Notes For Implementers

- Preserve this branch's current transcription-manager structure; do not replace large files with upstream versions wholesale
- Prefer minimal, localized diffs over broad refactors
- Stay strictly within transcription engines, model catalog, accelerator tuning, and the minimum settings/UI support needed for those features
- If a missing upstream helper appears necessary, port the smallest self-contained version of it instead of importing unrelated neighboring changes
