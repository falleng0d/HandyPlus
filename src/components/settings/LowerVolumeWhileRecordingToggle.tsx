import React from "react";
import { ToggleSwitch } from "../ui";
import { useSettings } from "../../hooks/useSettings";

interface LowerVolumeWhileRecordingToggleProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const LowerVolumeWhileRecordingToggle: React.FC<LowerVolumeWhileRecordingToggleProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();

    const lowerVolumeEnabled =
      getSetting("lower_volume_while_recording") ?? false;

    return (
      <ToggleSwitch
        checked={lowerVolumeEnabled}
        onChange={(enabled) =>
          updateSetting("lower_volume_while_recording", enabled)
        }
        isUpdating={isUpdating("lower_volume_while_recording")}
        label="Lower Volume While Recording"
        description="Automatically reduce system volume while Handy is recording, then restore it when finished."
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  });
