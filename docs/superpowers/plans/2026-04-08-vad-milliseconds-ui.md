# VAD Milliseconds UI Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Update the VAD settings UI so users edit prefill, hangover, and onset in milliseconds while the backend continues storing and receiving frame counts.

**Architecture:** Keep the backend unchanged. Convert frame-based settings to milliseconds in `VadSettings.tsx` for display, and convert milliseconds back to frames before calling the existing `updateSetting(...)` flow. Restrict input to multiples of `30 ms` so conversion stays exact.

**Tech Stack:** React, TypeScript, Zustand, Tauri

---

## File Map

- `src/components/settings/VadSettings.tsx`: VAD settings UI and ms<->frame conversion logic
- `docs/superpowers/specs/2026-04-08-vad-milliseconds-ui-design.md`: Approved design reference

### Task 1: Convert VAD UI From Frames To Milliseconds

**Files:**

- Modify: `src/components/settings/VadSettings.tsx`
- Reference: `docs/superpowers/specs/2026-04-08-vad-milliseconds-ui-design.md`

- [ ] **Step 1: Write the failing test or verification target**

This codebase does not currently expose a focused frontend test harness for this component. Use a behavior-first verification target instead:

- Default stored values:
  - `vad_prefill_frames = 15`
  - `vad_hangover_frames = 15`
  - `vad_onset_frames = 2`
- Expected UI values:
  - `450` for prefill
  - `450` for hangover
  - `60` for onset
- Expected save behavior:
  - entering `480` for prefill sends `16` frames via `updateSetting("vad_prefill_frames", 16)`
  - entering `90` for onset sends `3` frames via `updateSetting("vad_onset_frames", 3)`

- [ ] **Step 2: Verify current behavior is still frame-based**

Read `src/components/settings/VadSettings.tsx` and confirm these current behaviors before editing:

```tsx
const vadPrefillFrames = getSetting("vad_prefill_frames") ?? 15;
const vadHangoverFrames = getSetting("vad_hangover_frames") ?? 15;
const vadOnsetFrames = getSetting("vad_onset_frames") ?? 2;
```

Expected: the inputs currently display raw frame counts and pass raw integer values to `updateSetting(...)`.

- [ ] **Step 3: Write the minimal implementation**

Update `src/components/settings/VadSettings.tsx` to use a local frame-duration constant and convert UI values.

Add a local constant near the top of the component file:

```tsx
const VAD_FRAME_MS = 30;
```

Replace the displayed VAD time values with millisecond-derived values:

```tsx
const vadPrefillMs = (getSetting("vad_prefill_frames") ?? 15) * VAD_FRAME_MS;
const vadHangoverMs = (getSetting("vad_hangover_frames") ?? 15) * VAD_FRAME_MS;
const vadOnsetMs = (getSetting("vad_onset_frames") ?? 2) * VAD_FRAME_MS;
```

Update the handlers to convert milliseconds back to frames:

```tsx
const handlePrefillChange = async (
  event: React.ChangeEvent<HTMLInputElement>,
) => {
  const valueMs = parseInt(event.target.value, 10);
  if (!isNaN(valueMs) && valueMs >= 0 && valueMs % VAD_FRAME_MS === 0) {
    await updateSetting("vad_prefill_frames", valueMs / VAD_FRAME_MS);
  }
};
```

Apply the same pattern to hangover and onset.

Update the labels and descriptions:

```tsx
<span className="text-sm text-gray-400 w-32">Prefill (ms)</span>
<span className="text-sm text-gray-400 w-32">Hangover (ms)</span>
<span className="text-sm text-gray-400 w-32">Onset (ms)</span>
```

```tsx
<span className="text-xs text-gray-500">
  Milliseconds of audio to keep before speech starts
</span>
```

Set numeric input step sizes to `30`:

```tsx
<Input
  type="number"
  min="0"
  max="3000"
  step="30"
  value={vadPrefillMs}
  onChange={handlePrefillChange}
  disabled={isUpdating("vad_prefill_frames")}
  className="w-24"
/>
```

Use the same `step="30"` for hangover and onset. Keep threshold unchanged.

- [ ] **Step 4: Run verification**

Run:

```bash
bun check
```

Expected: pass with no TypeScript or Rust errors.

Then manually verify in the updated file that:

- `15` stored frames display as `450`
- `2` stored frames display as `60`
- only multiples of `30` are accepted by the save path
- backend setting keys remain:
  - `vad_prefill_frames`
  - `vad_hangover_frames`
  - `vad_onset_frames`

- [ ] **Step 5: Commit**

```bash
git add src/components/settings/VadSettings.tsx
git commit -m "feat: show VAD timing in milliseconds"
```
