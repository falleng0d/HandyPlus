import React from "react";
import { Button } from "../ui/Button";
import { Dropdown, DropdownOption } from "../ui/Dropdown";
import { PlayIcon } from "lucide-react";
import { SettingContainer } from "../ui/SettingContainer";
import { useSettingsStore } from "../../stores/settingsStore";
import { useSettings } from "../../hooks/useSettings";
import type { Settings } from "../../lib/types";

type SoundSettingKey = Extract<
  keyof Settings,
  "sound_theme" | "language_shortcut_sound_theme"
>;

interface SoundPickerProps {
  label: string;
  description: string;
  settingKey?: SoundSettingKey;
}

export const SoundPicker: React.FC<SoundPickerProps> = ({
  label,
  description,
  settingKey = "sound_theme",
}) => {
  const { getSetting, updateSetting } = useSettings();
  const playTestSound = useSettingsStore((state) => state.playTestSound);
  const customSounds = useSettingsStore((state) => state.customSounds);

  const selectedTheme = (getSetting(settingKey) ?? "marimba") as
    | "marimba"
    | "pop"
    | "custom";

  const options: DropdownOption[] = [
    { value: "marimba", label: "Marimba" },
    { value: "pop", label: "Pop" },
  ];

  // Only add Custom option if both custom sound files exist
  if (customSounds.start && customSounds.stop) {
    options.push({ value: "custom", label: "Custom" });
  }

  const handlePlayBothSounds = async () => {
    await playTestSound("start", selectedTheme);
    // Wait before playing stop sound
    await new Promise((resolve) => setTimeout(resolve, 800));
    await playTestSound("stop", selectedTheme);
  };

  return (
    <SettingContainer
      title={label}
      description={description}
      grouped
      layout="horizontal"
    >
      <div className="flex items-center gap-2">
        <Dropdown
          selectedValue={selectedTheme}
          onSelect={(value) =>
            updateSetting(settingKey, value as Settings[SoundSettingKey])
          }
          options={options}
        />
        <Button
          variant="ghost"
          size="sm"
          onClick={handlePlayBothSounds}
          title="Preview sound theme (plays start then stop)"
        >
          <PlayIcon className="h-4 w-4" />
        </Button>
      </div>
    </SettingContainer>
  );
};
