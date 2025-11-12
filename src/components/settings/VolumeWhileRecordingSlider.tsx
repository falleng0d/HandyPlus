import React from "react";
import { Slider } from "../ui/Slider";
import { useSettings } from "../../hooks/useSettings";

interface VolumeWhileRecordingSliderProps {
  disabled?: boolean;
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const VolumeWhileRecordingSlider: React.FC<
  VolumeWhileRecordingSliderProps
> = ({ disabled = false, descriptionMode = "tooltip", grouped = false }) => {
  const { getSetting, updateSetting } = useSettings();
  const volumeWhileRecording = getSetting("volume_while_recording") ?? 0.5;

  return (
    <Slider
      value={volumeWhileRecording}
      onChange={(value: number) =>
        updateSetting("volume_while_recording", value)
      }
      min={0}
      max={1}
      step={0.01}
      label="Volume While Recording"
      description="Set the volume level to reduce system audio to while recording"
      descriptionMode={descriptionMode}
      grouped={grouped}
      formatValue={(value) => `${Math.round(value * 100)}%`}
      disabled={disabled}
    />
  );
};
