import { toast } from "sonner";
import {
  commands,
  type AppSettings as Settings,
  type LanguageConfig,
  type SoundTheme,
  type Result,
} from "@/bindings";

/** Rejects so the store rolls the optimistic value back, and tells the user why. */
async function unwrapOrToast(pending: Promise<Result<null, string>>) {
  const result = await pending;
  if (result.status === "error") {
    toast.error(result.error);
    throw new Error(result.error);
  }
}

export const plusSettingUpdaters: {
  [K in keyof Settings]?: (value: Settings[K]) => Promise<unknown>;
} = {
  vad_threshold: (value) =>
    unwrapOrToast(commands.changeVadThresholdSetting(value as number)),
  vad_prefill_ms: (value) =>
    unwrapOrToast(commands.changeVadPrefillMsSetting(value as number)),
  vad_hangover_ms: (value) =>
    unwrapOrToast(commands.changeVadHangoverMsSetting(value as number)),
  vad_onset_ms: (value) =>
    unwrapOrToast(commands.changeVadOnsetMsSetting(value as number)),
  context_lines: (value) =>
    unwrapOrToast(commands.changeContextLinesSetting(value as number)),
  context_max_characters: (value) =>
    unwrapOrToast(
      commands.changeContextMaxCharactersSetting(value as number | null),
    ),
  context_max_paragraphs: (value) =>
    unwrapOrToast(commands.changeContextMaxParagraphsSetting(value as number)),
  lowercase_first_letter: (value) =>
    unwrapOrToast(commands.changeLowercaseFirstLetterSetting(value as boolean)),
  continuation_lowercase: (value) =>
    unwrapOrToast(
      commands.changeContinuationLowercaseSetting(value as boolean),
    ),
  remove_trailing_punctuation: (value) =>
    unwrapOrToast(
      commands.changeRemoveTrailingPunctuationSetting(value as boolean),
    ),
  lower_volume_while_recording: (value) =>
    unwrapOrToast(
      commands.changeLowerVolumeWhileRecordingSetting(value as boolean),
    ),
  volume_while_recording: (value) =>
    unwrapOrToast(commands.changeVolumeWhileRecordingSetting(value as number)),
  pause_dragon_when_dictating: (value) =>
    unwrapOrToast(
      commands.changePauseDragonWhenDictatingSetting(value as boolean),
    ),
  language_configs: (value) =>
    unwrapOrToast(commands.updateLanguageConfigs(value as LanguageConfig[])),
  language_cycle_shortcut: (value) =>
    unwrapOrToast(commands.updateLanguageCycleShortcut(value as string)),
  language_shortcut_sound_theme: (value) =>
    unwrapOrToast(
      commands.changeLanguageShortcutSoundThemeSetting(value as SoundTheme),
    ),
  typing_interval_ms: (value) =>
    unwrapOrToast(commands.changeTypingIntervalMsSetting(value as number)),
  on_recording_start_script: (value) =>
    unwrapOrToast(commands.changeRecordingStartScriptSetting(value as string)),
  on_recording_end_script: (value) =>
    unwrapOrToast(commands.changeRecordingEndScriptSetting(value as string)),
  custom_word_correction_enabled: (value) =>
    unwrapOrToast(
      commands.changeCustomWordCorrectionEnabledSetting(value as boolean),
    ),
  transcription_timeout_secs: (value) =>
    unwrapOrToast(
      commands.changeTranscriptionTimeoutSecsSetting(value as number),
    ),
  post_process_timeout_secs: (value) =>
    unwrapOrToast(
      commands.changePostProcessTimeoutSecsSetting(value as number),
    ),
};
