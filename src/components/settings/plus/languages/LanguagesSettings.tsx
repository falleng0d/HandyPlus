import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { PlayIcon, X } from "lucide-react";
import { commands, type LanguageConfig, type SoundTheme } from "@/bindings";
import { useSettings } from "../../../../hooks/useSettings";
import { useSettingsStore } from "../../../../stores/settingsStore";
import { useModelStore } from "../../../../stores/modelStore";
import { LANGUAGES } from "../../../../lib/constants/languages";
import { Button } from "../../../ui/Button";
import { Dropdown, type DropdownOption } from "../../../ui/Dropdown";
import { ResetButton } from "../../../ui/ResetButton";
import { SettingContainer } from "../../../ui/SettingContainer";
import { SettingsGroup } from "../../../ui/SettingsGroup";
import { ShortcutRecorder } from "./ShortcutRecorder";

const languageLabel = (code: string) =>
  LANGUAGES.find((language) => language.value === code)?.label ?? code;

export const LanguagesSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();
  const customSounds = useSettingsStore((state) => state.customSounds);
  const { models, downloadModel } = useModelStore();
  const [adding, setAdding] = useState<string | null>(null);

  const configs = getSetting("language_configs") ?? [];
  const cycleShortcut = getSetting("language_cycle_shortcut") ?? "";
  const soundTheme = getSetting("language_shortcut_sound_theme") ?? "pop";
  const prompts = getSetting("post_process_prompts") ?? [];
  const usesHandyKeys = getSetting("keyboard_implementation") !== "tauri";

  const saveConfigs = (next: LanguageConfig[]) =>
    updateSetting("language_configs", next);

  const updateConfig = (updated: LanguageConfig) =>
    saveConfigs(configs.map((c) => (c.id === updated.id ? updated : c)));

  const addLanguage = (language: string) => {
    setAdding(null);
    void saveConfigs([
      ...configs,
      {
        id: `lang_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`,
        language,
        shortcut_binding: "",
        prompt_id: null,
        model: null,
      },
    ]);
  };

  const selectModel = (config: LanguageConfig, modelId: string) => {
    void updateConfig({ ...config, model: modelId || null });
    const model = models.find((m) => m.id === modelId);
    if (model && !model.is_downloaded && !model.is_downloading) {
      void downloadModel(modelId);
    }
  };

  const usedLanguages = new Set(configs.map((c) => c.language));
  const addOptions = LANGUAGES.filter(
    (language) =>
      language.value !== "auto" && !usedLanguages.has(language.value),
  );

  const soundOptions: DropdownOption[] = [
    { value: "marimba", label: "Marimba" },
    { value: "pop", label: "Pop" },
  ];
  if (customSounds.start && customSounds.stop) {
    soundOptions.push({ value: "custom", label: "Custom" });
  }

  const modelOptions = useMemo<DropdownOption[]>(
    () => [
      { value: "", label: t("settings.languages.keepCurrent") },
      ...[...models]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((model) => ({
          value: model.id,
          label: model.is_downloaded
            ? model.name
            : t("settings.languages.needsDownload", { name: model.name }),
        })),
    ],
    [models, t],
  );

  const promptOptions: DropdownOption[] = [
    { value: "", label: t("settings.languages.keepCurrent") },
    ...prompts.map((prompt) => ({ value: prompt.id, label: prompt.name })),
  ];

  return (
    <div className="max-w-3xl w-full mx-auto space-y-6">
      <SettingsGroup
        title={t("settings.languages.title")}
        description={
          usesHandyKeys
            ? t("settings.languages.description")
            : t("settings.languages.needsHandyKeys")
        }
      >
        <SettingContainer
          title={t("settings.languages.cycle.title")}
          description={t("settings.languages.cycle.description")}
          descriptionMode="tooltip"
          layout="horizontal"
          grouped
        >
          <div className="flex items-center gap-1">
            <ShortcutRecorder
              value={cycleShortcut}
              disabled={!usesHandyKeys}
              onChange={(shortcut) =>
                updateSetting("language_cycle_shortcut", shortcut)
              }
            />
            <ResetButton
              onClick={() => updateSetting("language_cycle_shortcut", "")}
              disabled={!cycleShortcut || isUpdating("language_cycle_shortcut")}
              ariaLabel={t("settings.languages.clearShortcut")}
            />
          </div>
        </SettingContainer>
        <SettingContainer
          title={t("settings.languages.sound.title")}
          description={t("settings.languages.sound.description")}
          descriptionMode="tooltip"
          layout="horizontal"
          grouped
        >
          <div className="flex items-center gap-2">
            <Dropdown
              selectedValue={soundTheme}
              options={soundOptions}
              onSelect={(value) =>
                updateSetting(
                  "language_shortcut_sound_theme",
                  value as SoundTheme,
                )
              }
            />
            <Button
              variant="ghost"
              size="sm"
              onClick={() => commands.playLanguageShortcutSound()}
              title={t("settings.languages.sound.preview")}
            >
              <PlayIcon className="h-4 w-4" />
            </Button>
          </div>
        </SettingContainer>
        <SettingContainer
          title={t("settings.languages.add.title")}
          description={t("settings.languages.add.description")}
          descriptionMode="tooltip"
          layout="horizontal"
          grouped
        >
          <Dropdown
            selectedValue={adding}
            options={addOptions}
            onSelect={addLanguage}
            placeholder={t("settings.languages.add.placeholder")}
            disabled={addOptions.length === 0}
          />
        </SettingContainer>
      </SettingsGroup>

      {configs.length > 0 && (
        <SettingsGroup
          title={t("settings.languages.list.title")}
          description={t("settings.languages.list.description")}
        >
          {configs.map((config) => (
            <div key={config.id} className="flex items-center gap-2 px-4 py-2">
              <button
                type="button"
                aria-label={t("settings.languages.remove", {
                  language: languageLabel(config.language),
                })}
                className="p-1 rounded border border-transparent text-mid-gray hover:text-red-400 hover:bg-red-500/20 hover:border-red-500/50 cursor-pointer"
                onClick={() =>
                  saveConfigs(configs.filter((c) => c.id !== config.id))
                }
              >
                <X className="w-4 h-4" />
              </button>
              <span className="flex-1 min-w-0 truncate text-sm font-medium">
                {languageLabel(config.language)}
              </span>
              <Dropdown
                className="w-52"
                selectedValue={config.model ?? ""}
                options={modelOptions}
                onSelect={(value) => selectModel(config, value)}
              />
              <Dropdown
                className="w-52"
                selectedValue={config.prompt_id ?? ""}
                options={promptOptions}
                onSelect={(value) =>
                  updateConfig({ ...config, prompt_id: value || null })
                }
              />
              <ShortcutRecorder
                value={config.shortcut_binding ?? ""}
                disabled={!usesHandyKeys}
                onChange={(shortcut) =>
                  updateConfig({ ...config, shortcut_binding: shortcut })
                }
              />
              <ResetButton
                onClick={() =>
                  updateConfig({ ...config, shortcut_binding: "" })
                }
                disabled={!config.shortcut_binding}
                ariaLabel={t("settings.languages.clearShortcut")}
              />
            </div>
          ))}
        </SettingsGroup>
      )}
    </div>
  );
};
