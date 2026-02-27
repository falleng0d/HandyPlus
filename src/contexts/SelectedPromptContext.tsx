import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
} from "react";
import { listen } from "@tauri-apps/api/event";
import { useSettingsStore } from "../stores/settingsStore";

interface LanguageChangedEvent {
  language?: string;
  config_id?: string;
  prompt_id?: string | null;
}

interface SelectedPromptContextValue {
  selectedPromptId: string;
  setSelectedPromptId: (promptId: string) => Promise<void>;
  isUpdating: boolean;
}

const SelectedPromptContext = createContext<SelectedPromptContextValue | null>(
  null,
);

export const SelectedPromptProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const settings = useSettingsStore((state) => state.settings);
  const setSettings = useSettingsStore((state) => state.setSettings);
  const updateSetting = useSettingsStore((state) => state.updateSetting);
  const isUpdatingKey = useSettingsStore((state) => state.isUpdatingKey);

  const [selectedPromptId, setSelectedPromptIdState] = useState<string>(
    settings?.post_process_selected_prompt_id ?? "",
  );

  useEffect(() => {
    setSelectedPromptIdState(settings?.post_process_selected_prompt_id ?? "");
  }, [settings?.post_process_selected_prompt_id]);

  useEffect(() => {
    const unlisten = listen<LanguageChangedEvent>(
      "language-changed",
      (event) => {
        if (Object.prototype.hasOwnProperty.call(event.payload, "prompt_id")) {
          const promptId = event.payload.prompt_id ?? "";
          setSelectedPromptIdState(promptId);

          const currentSettings = useSettingsStore.getState().settings;
          setSettings(
            currentSettings
              ? {
                  ...currentSettings,
                  post_process_selected_prompt_id: promptId || null,
                }
              : currentSettings,
          );
        }
      },
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setSettings]);

  const setSelectedPromptId = useCallback(
    async (promptId: string) => {
      if (!promptId) {
        return;
      }

      setSelectedPromptIdState(promptId);
      await updateSetting("post_process_selected_prompt_id", promptId);
    },
    [updateSetting],
  );

  const value = useMemo<SelectedPromptContextValue>(
    () => ({
      selectedPromptId,
      setSelectedPromptId,
      isUpdating: isUpdatingKey("post_process_selected_prompt_id"),
    }),
    [isUpdatingKey, selectedPromptId, setSelectedPromptId],
  );

  return (
    <SelectedPromptContext.Provider value={value}>
      {children}
    </SelectedPromptContext.Provider>
  );
};

export const useSelectedPromptContext = (): SelectedPromptContextValue => {
  const context = useContext(SelectedPromptContext);
  if (!context) {
    throw new Error(
      "useSelectedPrompt must be used within a SelectedPromptProvider",
    );
  }
  return context;
};
