import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { NumberSetting } from "./NumberSetting";

export const PromptContextSettings: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <SettingsGroup
      title={t("settings.postProcessing.context.title")}
      description={t("settings.postProcessing.context.description")}
    >
      <NumberSetting
        title={t("settings.postProcessing.context.lines.title")}
        description={t("settings.postProcessing.context.lines.description")}
        value={getSetting("context_lines") ?? 30}
        min={0}
        max={1000}
        step={1}
        disabled={isUpdating("context_lines")}
        grouped
        onCommit={(value) => updateSetting("context_lines", value)}
      />
      <NumberSetting
        title={t("settings.postProcessing.context.paragraphs.title")}
        description={t(
          "settings.postProcessing.context.paragraphs.description",
        )}
        value={getSetting("context_max_paragraphs") ?? 0}
        min={0}
        max={1000}
        step={1}
        disabled={isUpdating("context_max_paragraphs")}
        grouped
        onCommit={(value) => updateSetting("context_max_paragraphs", value)}
      />
      <NumberSetting
        title={t("settings.postProcessing.context.maxCharacters.title")}
        description={t(
          "settings.postProcessing.context.maxCharacters.description",
        )}
        value={getSetting("context_max_characters") ?? 0}
        min={0}
        max={100000}
        step={100}
        disabled={isUpdating("context_max_characters")}
        grouped
        onCommit={(value) =>
          updateSetting("context_max_characters", value === 0 ? null : value)
        }
      />
    </SettingsGroup>
  );
};
