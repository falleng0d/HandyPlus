import React, { useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";
import { ShortcutRecorder } from "../../ui/ShortcutRecorder";
import { Dropdown } from "../../ui";
import { Select } from "../../ui/Select";
import { ResetButton } from "../../ui/ResetButton";
import { useSettings } from "../../../hooks/useSettings";
import { LANGUAGES } from "../../../lib/constants/languages";
import type { LanguageConfig } from "../../../lib/types";

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
  const { settings, postProcessModelOptions } = useSettings();

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

  const providerId = settings?.post_process_provider_id ?? "openai";
  const rawModelOptions = postProcessModelOptions[providerId] ?? [];
  const modelOptions = useMemo(() => {
    const seen = new Set<string>();
    const options: { value: string; label: string }[] = [];
    for (const m of rawModelOptions) {
      if (m && !seen.has(m)) {
        seen.add(m);
        options.push({ value: m, label: m });
      }
    }
    // Ensure current value is present
    if (config.model && !seen.has(config.model)) {
      options.push({ value: config.model, label: config.model });
    }
    return options;
  }, [rawModelOptions, config.model]);

  const handleModelChange = (value: string | null) => {
    onUpdate({ ...config, model: value ?? null });
  };

  const handleModelCreate = (value: string) => {
    const trimmed = value.trim();
    if (!trimmed) return;
    onUpdate({ ...config, model: trimmed });
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

      {/* Model override */}
      <Select
        value={config.model ?? null}
        options={modelOptions}
        onChange={handleModelChange}
        onCreateOption={handleModelCreate}
        placeholder="Default"
        isCreatable
        formatCreateLabel={(input) => `Use "${input}"`}
        className="w-32 text-sm"
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
