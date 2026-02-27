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
}

interface SelectedLanguageContextValue {
  selectedLanguage: string;
  setSelectedLanguage: (language: string) => Promise<void>;
  resetSelectedLanguage: () => Promise<void>;
  isUpdating: boolean;
}

const SelectedLanguageContext =
  createContext<SelectedLanguageContextValue | null>(null);

export const SelectedLanguageProvider: React.FC<{
  children: React.ReactNode;
}> = ({ children }) => {
  const settings = useSettingsStore((state) => state.settings);
  const setSettings = useSettingsStore((state) => state.setSettings);
  const updateSetting = useSettingsStore((state) => state.updateSetting);
  const resetSetting = useSettingsStore((state) => state.resetSetting);
  const isUpdatingKey = useSettingsStore((state) => state.isUpdatingKey);

  const [selectedLanguage, setSelectedLanguageState] = useState<string>(
    settings?.selected_language ?? "auto",
  );

  useEffect(() => {
    setSelectedLanguageState(settings?.selected_language ?? "auto");
  }, [settings?.selected_language]);

  useEffect(() => {
    const unlisten = listen<LanguageChangedEvent>(
      "language-changed",
      (event) => {
        const language = event.payload.language;
        if (!language) {
          return;
        }

        setSelectedLanguageState(language);
        const currentSettings = useSettingsStore.getState().settings;
        setSettings(
          currentSettings
            ? {
                ...currentSettings,
                selected_language: language,
              }
            : currentSettings,
        );
      },
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [setSettings]);

  const setSelectedLanguage = useCallback(
    async (language: string) => {
      if (!language) {
        return;
      }

      setSelectedLanguageState(language);
      await updateSetting("selected_language", language);
    },
    [updateSetting],
  );

  const resetSelectedLanguage = useCallback(async () => {
    setSelectedLanguageState("auto");
    await resetSetting("selected_language");
  }, [resetSetting]);

  const value = useMemo<SelectedLanguageContextValue>(
    () => ({
      selectedLanguage,
      setSelectedLanguage,
      resetSelectedLanguage,
      isUpdating: isUpdatingKey("selected_language"),
    }),
    [
      isUpdatingKey,
      resetSelectedLanguage,
      selectedLanguage,
      setSelectedLanguage,
    ],
  );

  return (
    <SelectedLanguageContext.Provider value={value}>
      {children}
    </SelectedLanguageContext.Provider>
  );
};

export const useSelectedLanguageContext = (): SelectedLanguageContextValue => {
  const context = useContext(SelectedLanguageContext);
  if (!context) {
    throw new Error(
      "useSelectedLanguageContext must be used within a SelectedLanguageProvider",
    );
  }
  return context;
};
