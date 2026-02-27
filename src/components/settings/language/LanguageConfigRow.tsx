import React, { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { ShortcutRecorder } from "../../ui/ShortcutRecorder";
import { Dropdown } from "../../ui";
import { ResetButton } from "../../ui/ResetButton";
import { useSettings } from "../../../hooks/useSettings";
import { LANGUAGES } from "../../../lib/constants/languages";
import type { LanguageConfig } from "../../../lib/types";
import { useModelsContext } from "../../../contexts/ModelsContext.tsx";

interface LanguageConfigRowProps {
  config: LanguageConfig;
  onUpdate: (config: LanguageConfig) => void;
  onRemove: (id: string) => void;
}

export const LanguageConfigRow: React.FC<LanguageConfigRowProps> = ({
  config,
  onUpdate,
  onRemove,
}) => {
  const { settings } = useSettings();
  const { models, downloadModel } = useModelsContext();

  const languageLabel =
    LANGUAGES.find((l) => l.value === config.language)?.label ??
    config.language;

  const prompts = settings?.post_process_prompts ?? [];
  const promptOptions = useMemo(
    () => [
      { value: "", label: "Default" },
      ...prompts.map((p) => ({ value: p.id, label: p.name })),
    ],
    [prompts],
  );

  const modelOptions = useMemo(() => {
    const options: { value: string; label: string }[] = [
      { value: "", label: "Default" },
      ...[...models]
        .sort((a, b) => a.name.localeCompare(b.name))
        .map((model) => ({
          value: model.id,
          label: model.is_downloaded ? model.name : `${model.name} (download)`,
        })),
    ];

    if (
      config.model &&
      !options.some((option) => option.value === config.model)
    ) {
      options.push({ value: config.model, label: config.model });
    }

    return options;
  }, [models, config.model]);

  const handleModelChange = async (value: string) => {
    const modelId = value === "" ? null : value;
    onUpdate({ ...config, model: modelId });

    if (!modelId) {
      return;
    }

    const selected = models.find((model) => model.id === modelId);
    if (selected && !selected.is_downloaded && !selected.is_downloading) {
      await downloadModel(modelId, { activateAfterDownload: false });
    }
  };

  const handlePromptChange = (value: string) => {
    onUpdate({ ...config, prompt_id: value === "" ? null : value });
  };

  const handleShortcutChange = (shortcut: string) => {
    onUpdate({ ...config, shortcut_binding: shortcut });
  };

  const handleShortcutSuspend = async () => {
    await invoke("suspend_language_shortcut", { configId: config.id });
  };

  const handleShortcutResume = async () => {
    await invoke("resume_language_shortcut", { configId: config.id });
  };

  const handleReset = () => {
    onUpdate({ ...config, model: null, prompt_id: null, shortcut_binding: "" });
  };

  const hasOverrides =
    config.model !== null ||
    config.prompt_id !== null ||
    config.shortcut_binding !== "";

  return (
    <div className="flex items-center gap-2 px-4 py-2">
      {/* Remove button */}
      <button
        type="button"
        aria-label={`Remove ${languageLabel}`}
        className="p-1 rounded border border-transparent hover:bg-red-500/20 hover:border-red-500/50 text-mid-gray hover:text-red-400 transition-all duration-150 cursor-pointer"
        onClick={() => onRemove(config.id)}
      >
        <X className="w-4 h-4" />
      </button>

      {/* Language label */}
      <span className="text-sm font-medium w-24 flex-1">{languageLabel}</span>

      {/* Dictation model override */}
      <Dropdown
        selectedValue={config.model ?? ""}
        options={modelOptions}
        onSelect={(value) => {
          void handleModelChange(value);
        }}
        placeholder="Default"
        className="w-40"
        wide={false}
        buttonClassName="h-10"
      />

      {/* Prompt override */}
      <Dropdown
        selectedValue={config.prompt_id ?? ""}
        options={promptOptions}
        onSelect={handlePromptChange}
        placeholder="Default"
        className="w-24 max-w-[90px]"
        wide={false}
        buttonClassName="h-10"
      />

      {/* Shortcut recorder */}
      <ShortcutRecorder
        value={config.shortcut_binding}
        onChange={handleShortcutChange}
        placeholder="No shortcut"
        onSuspend={handleShortcutSuspend}
        onResume={handleShortcutResume}
        className="h-10"
      />

      {/* Reset overrides */}
      <ResetButton
        onClick={handleReset}
        disabled={!hasOverrides}
        ariaLabel="Reset overrides"
      />
    </div>
  );
};
