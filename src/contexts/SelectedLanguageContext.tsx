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

  const updateSelectedLanguageInSettings = useCallback(
    (language: string) => {
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
    [setSettings],
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
        updateSelectedLanguageInSettings(language);
      },
    );

    return () => {
      unlisten.then((fn) => fn());
    };
  }, [updateSelectedLanguageInSettings]);

  const setSelectedLanguage = useCallback(
    async (language: string) => {
      if (!language) {
        return;
      }

      setSelectedLanguageState(language);
      updateSelectedLanguageInSettings(language);
      await updateSetting("selected_language", language);
    },
    [updateSelectedLanguageInSettings, updateSetting],
  );

  const resetSelectedLanguage = useCallback(async () => {
    setSelectedLanguageState("auto");
    updateSelectedLanguageInSettings("auto");
    await resetSetting("selected_language");
  }, [resetSetting, updateSelectedLanguageInSettings]);

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
