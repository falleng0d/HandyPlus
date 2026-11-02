import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { NumberSetting } from "./NumberSetting";

interface Props {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const AutoStopSilence: React.FC<Props> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <NumberSetting
      title={t("settings.general.autoStopSilence.title")}
      description={t("settings.general.autoStopSilence.description")}
      value={getSetting("auto_stop_silence_secs") ?? 0}
      min={0}
      max={600}
      step={0.5}
      unit={t("settings.advanced.seconds")}
      disabled={isUpdating("auto_stop_silence_secs")}
      descriptionMode={descriptionMode}
      grouped={grouped}
      onCommit={(value) => updateSetting("auto_stop_silence_secs", value)}
    />
  );
};
