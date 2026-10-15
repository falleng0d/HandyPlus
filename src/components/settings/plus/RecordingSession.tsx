import React from "react";
import { useTranslation } from "react-i18next";
import { useSettings } from "../../../hooks/useSettings";
import { SettingsGroup } from "../../ui/SettingsGroup";
import { ToggleSwitch } from "../../ui/ToggleSwitch";
import { TextSetting } from "./TextSetting";

export const RecordingSession: React.FC = () => {
  const { t } = useTranslation();
  const { getSetting, updateSetting, isUpdating } = useSettings();

  return (
    <SettingsGroup
      title={t("settings.advanced.recordingSession.title")}
      description={t("settings.advanced.recordingSession.description")}
    >
      <TextSetting
        title={t("settings.advanced.recordingSession.startScript.title")}
        description={t(
          "settings.advanced.recordingSession.startScript.description",
        )}
        value={getSetting("on_recording_start_script") ?? ""}
        placeholder={t("settings.advanced.recordingSession.placeholder")}
        disabled={isUpdating("on_recording_start_script")}
        grouped
        onCommit={(value) => updateSetting("on_recording_start_script", value)}
      />
      <TextSetting
        title={t("settings.advanced.recordingSession.endScript.title")}
        description={t(
          "settings.advanced.recordingSession.endScript.description",
        )}
        value={getSetting("on_recording_end_script") ?? ""}
        placeholder={t("settings.advanced.recordingSession.placeholder")}
        disabled={isUpdating("on_recording_end_script")}
        grouped
        onCommit={(value) => updateSetting("on_recording_end_script", value)}
      />
      <ToggleSwitch
        checked={getSetting("pause_dragon_when_dictating") ?? false}
        onChange={(value) =>
          updateSetting("pause_dragon_when_dictating", value)
        }
        isUpdating={isUpdating("pause_dragon_when_dictating")}
        label={t("settings.advanced.recordingSession.pauseDragon.title")}
        description={t(
          "settings.advanced.recordingSession.pauseDragon.description",
        )}
        grouped
      />
    </SettingsGroup>
  );
};
