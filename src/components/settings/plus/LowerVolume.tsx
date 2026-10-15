import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { Slider } from "../../ui/Slider";
import { ToggleSwitch } from "../../ui/ToggleSwitch";

interface LowerVolumeProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const LowerVolume: React.FC<LowerVolumeProps> = ({
  descriptionMode = "tooltip",
  grouped = false,
}) => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const enabled = getSetting("lower_volume_while_recording") ?? false;

  return (
    <>
      <ToggleSwitch
        checked={enabled}
        onChange={(value) =>
          updateSetting("lower_volume_while_recording", value)
        }
        isUpdating={isUpdating("lower_volume_while_recording")}
        label={t("settings.sound.lowerVolume.title")}
        description={t("settings.sound.lowerVolume.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
      <Slider
        value={getSetting("volume_while_recording") ?? 0.3}
        onChange={(value) => updateSetting("volume_while_recording", value)}
        min={0}
        max={1}
        step={0.01}
        disabled={!enabled}
        label={t("settings.sound.lowerVolume.level.title")}
        description={t("settings.sound.lowerVolume.level.description")}
        descriptionMode={descriptionMode}
        grouped={grouped}
        formatValue={(value) => `${Math.round(value * 100)}%`}
      />
    </>
  );
};
