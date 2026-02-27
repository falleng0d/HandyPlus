import React, { useCallback, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { SettingsGroup } from "../../ui";
import { SettingContainer } from "../../ui";
import { Dropdown } from "../../ui";
import { ShortcutRecorder } from "../../ui/ShortcutRecorder";
import { ResetButton } from "../../ui/ResetButton";
import { LanguageConfigRow } from "./LanguageConfigRow";
import { useSettings } from "../../../hooks/useSettings";
import { LANGUAGES } from "../../../lib/constants/languages";
import type { LanguageConfig } from "../../../lib/types";

export const LanguageShortcutsSettings: React.FC = () => {
  const { settings, updateSetting, isUpdating } = useSettings();
  const [addLanguageValue, setAddLanguageValue] = useState<string | null>(null);

  const languageConfigs = settings?.language_configs ?? [];
  const cycleShortcut = settings?.language_cycle_shortcut ?? "";

  // Languages not yet added
  const usedLanguages = new Set(languageConfigs.map((c) => c.language));
  const availableLanguages = LANGUAGES.filter(
    (l) => !usedLanguages.has(l.value),
  ).map((l) => ({ value: l.value, label: l.label }));

  const handleAddLanguage = useCallback(
    (languageValue: string) => {
      if (!languageValue) return;
      const newConfig: LanguageConfig = {
        id: `lang_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        language: languageValue,
        shortcut_binding: "",
        prompt_id: null,
        model: null,
      };
      const updated = [...languageConfigs, newConfig];
      void updateSetting("language_configs", updated);
      // Clear selection immediately
      setAddLanguageValue(null);
    },
    [languageConfigs, updateSetting],
  );

  const handleUpdateConfig = useCallback(
    (updated: LanguageConfig) => {
      const newConfigs = languageConfigs.map((c) =>
        c.id === updated.id ? updated : c,
      );
      void updateSetting("language_configs", newConfigs);
    },
    [languageConfigs, updateSetting],
  );

  const handleRemoveConfig = useCallback(
    (id: string) => {
      const newConfigs = languageConfigs.filter((c) => c.id !== id);
      void updateSetting("language_configs", newConfigs);
    },
    [languageConfigs, updateSetting],
  );

  const handleCycleShortcutChange = useCallback(
    (shortcut: string) => {
      void updateSetting("language_cycle_shortcut", shortcut);
    },
    [updateSetting],
  );

  const handleCycleShortcutReset = useCallback(() => {
    void updateSetting("language_cycle_shortcut", "");
  }, [updateSetting]);

  const handleCycleShortcutSuspend = async () => {
    await invoke("suspend_language_cycle_shortcut");
  };

  const handleCycleShortcutResume = async () => {
    await invoke("resume_language_cycle_shortcut");
  };

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup title="Language Cycling">
        <SettingContainer
          title="Cycle Languages"
          description="Press this shortcut to cycle through your configured languages in order."
          descriptionMode="tooltip"
          layout="horizontal"
          grouped={true}
        >
          <div className="flex items-center gap-1">
            <ShortcutRecorder
              value={cycleShortcut}
              onChange={handleCycleShortcutChange}
              placeholder="No shortcut"
              onSuspend={handleCycleShortcutSuspend}
              onResume={handleCycleShortcutResume}
            />
            <ResetButton
              onClick={handleCycleShortcutReset}
              disabled={
                cycleShortcut.trim() === "" ||
                isUpdating("language_cycle_shortcut")
              }
              ariaLabel="Reset cycle language shortcut"
            />
          </div>
        </SettingContainer>

        <SettingContainer
          title="Add Language"
          description="Select a language to add to your language list."
          descriptionMode="tooltip"
          layout="horizontal"
          grouped={true}
        >
          <Dropdown
            selectedValue={addLanguageValue}
            options={availableLanguages}
            onSelect={(value) => {
              handleAddLanguage(value);
            }}
            placeholder="Select a language..."
            disabled={availableLanguages.length === 0}
            className="min-w-[180px]"
          />
        </SettingContainer>
      </SettingsGroup>

      {languageConfigs.length > 0 && (
        <SettingsGroup title="Languages">
          <div className="space-y-1">
            <div className="flex items-center gap-2 px-4 py-1 text-xs text-mid-gray/70">
              <div className="w-6 shrink-0" />
              <span className="w-24 flex-1">Language</span>
              <span className="w-32 text-center">Model</span>
              <span className="w-32 text-center">Prompt</span>
              <span className="w-28 text-center">Shortcut</span>
              <div className="w-7" />
            </div>
            {languageConfigs.map((config) => (
              <LanguageConfigRow
                key={config.id}
                config={config}
                onUpdate={handleUpdateConfig}
                onRemove={handleRemoveConfig}
              />
            ))}
          </div>
        </SettingsGroup>
      )}
    </div>
  );
};
