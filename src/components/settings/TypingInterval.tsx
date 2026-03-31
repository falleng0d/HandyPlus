import React from "react";
import { useSettings } from "../../hooks/useSettings";
import { Input } from "../ui/Input";
import { SettingContainer } from "../ui/SettingContainer";

interface TypingIntervalProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

export const TypingInterval: React.FC<TypingIntervalProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { getSetting, updateSetting, isUpdating } = useSettings();

  const typingInterval = getSetting("typing_interval_ms") ?? 0;

  const handleChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const value = parseInt(event.target.value, 10);
    if (!isNaN(value) && value >= 0) {
      await updateSetting("typing_interval_ms", value);
    }
  };

  return (
    <SettingContainer
      title="Typing Interval (ms)"
      description="Delay between each character when using Direct paste method. Set to 0 for instant typing."
      descriptionMode={descriptionMode}
      grouped={grouped}
      layout="horizontal"
    >
      <Input
        type="number"
        min="0"
        max="100"
        value={typingInterval}
        onChange={handleChange}
        disabled={isUpdating("typing_interval_ms")}
        className="w-24"
      />
    </SettingContainer>
  );
};
