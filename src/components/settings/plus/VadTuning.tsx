import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { NumberSetting } from "./NumberSetting";

interface VadTuningProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const VadTuning: React.FC<VadTuningProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  if (!(getSetting("vad_enabled") ?? true)) {
    return null;
  }

  const ms = t("settings.advanced.vadTuning.milliseconds");
  const durations = [
    { key: "vad_prefill_ms", name: "prefill" },
    { key: "vad_hangover_ms", name: "hangover" },
    { key: "vad_onset_ms", name: "onset" },
  ] as const;

  return (
    <>
      {(getSetting("vad_backend") ?? "silero") === "silero" && (
        <NumberSetting
          title={t("settings.advanced.vadTuning.threshold.title")}
          description={t("settings.advanced.vadTuning.threshold.description")}
          value={getSetting("vad_threshold") ?? 0.3}
          min={0}
          max={1}
          step={0.05}
          disabled={isUpdating("vad_threshold")}
          descriptionMode={descriptionMode}
          grouped={grouped}
          onCommit={(value) => updateSetting("vad_threshold", value)}
        />
      )}
      {durations.map(({ key, name }) => (
        <NumberSetting
          key={key}
          title={t(`settings.advanced.vadTuning.${name}.title`)}
          description={t(`settings.advanced.vadTuning.${name}.description`)}
          value={getSetting(key) ?? 0}
          min={0}
          max={3000}
          step={30}
          unit={ms}
          disabled={isUpdating(key)}
          descriptionMode={descriptionMode}
          grouped={grouped}
          onCommit={(value) => updateSetting(key, value)}
        />
      ))}
    </>
  );
};
