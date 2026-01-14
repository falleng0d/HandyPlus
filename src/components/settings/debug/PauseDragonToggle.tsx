import React from "react";
import { ToggleSwitch } from "../../ui";
import { useSettings } from "../../../hooks/useSettings";

interface PauseDragonToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const PauseDragonToggle: React.FC<PauseDragonToggleProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const pauseDragon = getSetting("pause_dragon_when_dictating") || false;

    return (
      <ToggleSwitch
        checked={pauseDragon}
        onChange={(enabled) =>
          updateSetting("pause_dragon_when_dictating", enabled)
        }
        isUpdating={isUpdating("pause_dragon_when_dictating")}
        label="Pause Dragon when Dictating"
        description="Automatically pauses Dragon Nuance dictation when Handy is recording, and resumes it afterwards."
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
