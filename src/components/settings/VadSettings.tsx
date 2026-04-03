import React from "react";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";

interface VadSettingsProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const VadSettings: React.FC<VadSettingsProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const vadThreshold = getSetting("vad_threshold") ?? 0.3;
  const vadPrefillFrames = getSetting("vad_prefill_frames") ?? 15;
  const vadHangoverFrames = getSetting("vad_hangover_frames") ?? 15;
  const vadOnsetFrames = getSetting("vad_onset_frames") ?? 2;

  const handleThresholdChange = async (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value = parseFloat(event.target.value);
    if (!isNaN(value) && value >= 0 && value <= 1) {
      await updateSetting("vad_threshold", value);
    }
  };

  const handlePrefillChange = async (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value = parseInt(event.target.value, 10);
    if (!isNaN(value) && value >= 0) {
      await updateSetting("vad_prefill_frames", value);
    }
  };

  const handleHangoverChange = async (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value = parseInt(event.target.value, 10);
    if (!isNaN(value) && value >= 0) {
      await updateSetting("vad_hangover_frames", value);
    }
  };

  const handleOnsetChange = async (
    event: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const value = parseInt(event.target.value, 10);
    if (!isNaN(value) && value >= 0) {
      await updateSetting("vad_onset_frames", value);
    }
  };

  return (
    <SettingContainer
      title="Voice Activity Detection (VAD)"
      description="Configure the Silero VAD engine for speech detection. Threshold controls sensitivity, while prefill/hangover/onset frames help filter noise."
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="stacked"
    >
      <div className="space-y-3 w-full">
        <div className="flex items-center gap-4">
          <span className="text-sm text-gray-400 w-32">Threshold</span>
          <Input
            type="number"
            min="0"
            max="1"
            step="0.1"
            value={vadThreshold}
            onChange={handleThresholdChange}
            disabled={isUpdating("vad_threshold")}
            className="w-24"
          />
          <span className="text-xs text-gray-500">
            Probability above which audio is speech (0-1)
          </span>
        </div>
        <div className="flex items-center gap-4">
          <span className="text-sm text-gray-400 w-32">Prefill Frames</span>
          <Input
            type="number"
            min="0"
            max="100"
            value={vadPrefillFrames}
            onChange={handlePrefillChange}
            disabled={isUpdating("vad_prefill_frames")}
            className="w-24"
          />
          <span className="text-xs text-gray-500">
            Frames to keep before speech starts
          </span>
        </div>
        <div className="flex items-center gap-4">
          <span className="text-sm text-gray-400 w-32">Hangover Frames</span>
          <Input
            type="number"
            min="0"
            max="100"
            value={vadHangoverFrames}
            onChange={handleHangoverChange}
            disabled={isUpdating("vad_hangover_frames")}
            className="w-24"
          />
          <span className="text-xs text-gray-500">
            Frames to keep after speech ends
          </span>
        </div>
        <div className="flex items-center gap-4">
          <span className="text-sm text-gray-400 w-32">Onset Frames</span>
          <Input
            type="number"
            min="0"
            max="100"
            value={vadOnsetFrames}
            onChange={handleOnsetChange}
            disabled={isUpdating("vad_onset_frames")}
            className="w-24"
          />
          <span className="text-xs text-gray-500">
            Consecutive voice frames needed to detect speech
          </span>
        </div>
      </div>
    </SettingContainer>
  );
};
