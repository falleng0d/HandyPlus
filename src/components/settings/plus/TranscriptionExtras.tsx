import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { NumberSetting } from "./NumberSetting";

interface Props {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const CustomWordCorrectionToggle: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <ToggleSwitch
      checked={getSetting("custom_word_correction_enabled") ?? true}
      onChange={(enabled) =>
        updateSetting("custom_word_correction_enabled", enabled)
      }
      isUpdating={isUpdating("custom_word_correction_enabled")}
      label={t("settings.advanced.customWordCorrection.title")}
      description={t("settings.advanced.customWordCorrection.description")}
      descriptionMode={descriptionMode}
      grouped={grouped}
    />
  );
};

type FinishingToggleKey =
  | "lowercase_first_letter"
  | "continuation_lowercase"
  | "remove_trailing_punctuation";

const FINISHING_TOGGLES: { key: FinishingToggleKey; label: string }[] = [
  { key: "lowercase_first_letter", label: "lowercaseFirstLetter" },
  { key: "continuation_lowercase", label: "continuationLowercase" },
  { key: "remove_trailing_punctuation", label: "removeTrailingPunctuation" },
];

/** Toggles that change the final text after post-processing. */
export const TextFinishing: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <>
      {FINISHING_TOGGLES.map(({ key, label }) => (
        <ToggleSwitch
          key={key}
          checked={getSetting(key) ?? false}
          onChange={(enabled) => updateSetting(key, enabled)}
          isUpdating={isUpdating(key)}
          label={t(`settings.advanced.${label}.title`)}
          description={t(`settings.advanced.${label}.description`)}
          descriptionMode={descriptionMode}
          grouped={grouped}
        />
      ))}
    </>
  );
};

export const TypingInterval: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <NumberSetting
      title={t("settings.advanced.typingInterval.title")}
      description={t("settings.advanced.typingInterval.description")}
      value={getSetting("typing_interval_ms") ?? 0}
      min={0}
      max={1000}
      step={5}
      unit={t("settings.advanced.vadTuning.milliseconds")}
      disabled={isUpdating("typing_interval_ms")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      onCommit={(value) => updateSetting("typing_interval_ms", value)}
    />
  );
};

export const TranscriptionTimeout: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <NumberSetting
      title={t("settings.advanced.transcriptionTimeout.title")}
      description={t("settings.advanced.transcriptionTimeout.description")}
      value={getSetting("transcription_timeout_secs") ?? 120}
      min={0}
      max={3600}
      step={10}
      unit={t("settings.advanced.seconds")}
      disabled={isUpdating("transcription_timeout_secs")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      onCommit={(value) => updateSetting("transcription_timeout_secs", value)}
    />
  );
};

export const PostProcessTimeout: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <NumberSetting
      title={t("settings.postProcessing.timeout.title")}
      description={t("settings.postProcessing.timeout.description")}
      value={getSetting("post_process_timeout_secs") ?? 15}
      min={0}
      max={600}
      step={5}
      unit={t("settings.advanced.seconds")}
      disabled={isUpdating("post_process_timeout_secs")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      onCommit={(value) => updateSetting("post_process_timeout_secs", value)}
    />
  );
};
