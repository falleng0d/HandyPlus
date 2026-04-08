# VAD Milliseconds UI Design

**Goal**

Change the frontend VAD settings UI so users edit time-based values in milliseconds instead of raw frame counts, while keeping the backend storage and command API in frames.

## Current State

- The VAD backend processes audio in fixed `30 ms` frames.
- `vad_prefill_frames`, `vad_hangover_frames`, and `vad_onset_frames` are stored in settings as frame counts.
- `src/components/settings/VadSettings.tsx` currently displays and edits those values directly as frames.

## Chosen Approach

Keep the backend unchanged and adapt only the frontend VAD settings component:

- Read frame-based values from settings.
- Display them as milliseconds using `frames * 30`.
- When the user edits a value, convert back to frames using `ms / 30` before calling `updateSetting(...)`.
- Restrict input to multiples of `30 ms` so conversion is exact and predictable.

This keeps the change minimal, avoids backend migration, and matches the current VAD implementation.

## Scope

### In Scope

- `src/components/settings/VadSettings.tsx`
- UI labels and helper text for prefill, hangover, and onset settings
- Frontend conversion between milliseconds and frames

### Out of Scope

- Rust backend changes
- Settings schema changes
- Tauri command changes
- Changing threshold behavior
- Displaying both milliseconds and frames unless needed later

## UI Behavior

### Threshold

- Keep as-is.
- No unit conversion.

### Prefill / Hangover / Onset

- Rename labels to indicate milliseconds:
  - `Prefill (ms)`
  - `Hangover (ms)`
  - `Onset (ms)`
- Display converted values from stored frames.
- Use numeric inputs with `step="30"`.
- Keep existing lower bounds of `0`.
- Preserve the current maximum input shape unless there is a concrete reason to relax it.

### Conversion Rules

- Display: `displayMs = frames * 30`
- Save: `frames = ms / 30`
- Because the UI restricts input to multiples of `30`, no rounding logic is needed.

## Implementation Notes

- Add a small local constant in `VadSettings.tsx`, for example `const VAD_FRAME_MS = 30`.
- Use that constant for all conversions so the intent is explicit.
- Keep the existing `updateSetting(...)` flow.
- Avoid introducing new shared helpers unless the file becomes meaningfully more complex.

## Validation

- Confirm default values render as:
  - prefill `450 ms` from `15` frames
  - hangover `450 ms` from `15` frames
  - onset `60 ms` from `2` frames
- Confirm editing a multiple of `30 ms` updates the matching frame count in settings.
- Run the relevant frontend check, and `bun check` if practical.

## Risks

- If users manually type a non-multiple of `30`, browser number inputs may still allow transient invalid text while editing.
- The save path should continue to ignore invalid numeric input until a valid number is present.

## Success Criteria

- Users no longer need to reason about VAD frames in the settings UI.
- Backend behavior remains unchanged.
- Displayed values map exactly to stored values with no rounding ambiguity.
