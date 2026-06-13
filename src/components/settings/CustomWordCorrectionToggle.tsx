import React from "react";
import { useSettings } from "../../hooks/useSettings";
import { ToggleSwitch } from "../ui/ToggleSwitch";

interface CustomWordCorrectionToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const CustomWordCorrectionToggle: React.FC<CustomWordCorrectionToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const enabled = getSetting("custom_word_correction_enabled") ?? true;

    return (
      <ToggleSwitch
        checked={enabled}
        onChange={(enabled) =>
          updateSetting("custom_word_correction_enabled", enabled)
        }
        isUpdating={isUpdating("custom_word_correction_enabled")}
        label="Custom Word Correction"
        description="Automatically correct similar-sounding transcription words to entries in your custom words list."
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
