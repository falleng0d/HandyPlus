import React, { useMemo } from "react";
import { ToggleSwitch } from "../ui";
import { useSettings } from "../../hooks/useSettings";
import { useModelsContext } from "../../contexts/ModelsContext";

interface TranslateToEnglishProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

const unsupportedTranslationModels = [
  "parakeet-tdt-0.6b-v2",
  "parakeet-tdt-0.6b-v3",
  "turbo",
];

export const TranslateToEnglish: React.FC<TranslateToEnglishProps> = React.memo(
  ({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const { currentModel, models } = useModelsContext();

    const translateToEnglish = getSetting("translate_to_english") || false;
    const isDisabledTranslation =
      unsupportedTranslationModels.includes(currentModel);

    const description = useMemo(() => {
      if (isDisabledTranslation) {
        const currentModelDisplayName = models.find(
          (model) => model.id === currentModel,
        )?.name;
        return `Translation is not supported by the ${currentModelDisplayName} model.`;
      }

      return "Automatically translate speech from other languages to English during transcription.";
    }, [models, currentModel, isDisabledTranslation]);

    return (
      <ToggleSwitch
        checked={translateToEnglish}
        onChange={(enabled) => updateSetting("translate_to_english", enabled)}
        isUpdating={isUpdating("translate_to_english")}
        disabled={isDisabledTranslation}
        label="Translate to English"
        description={description}
        descriptionMode={descriptionMode}
        grouped={grouped}
      />
    );
  },
);
