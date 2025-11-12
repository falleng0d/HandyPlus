import React, { useEffect, useState } from "react";
import { SettingContainer, Textarea } from "../ui";
import { useSettings } from "../../hooks/useSettings";

interface OnRecordingEndScriptInputProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const OnRecordingEndScriptInput: React.FC<OnRecordingEndScriptInputProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [draft, setDraft] = useState("");

    const script = getSetting("on_recording_end_script") ?? "";

    useEffect(() => {
      setDraft(script);
    }, [script]);

    return (
      <SettingContainer
        title="On Recording End Script"
        description={`Execute a one-liner script when recording ends. Examples: python 'print("done")' or echo 'Recording finished'`}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={(e) =>
            updateSetting("on_recording_end_script", e.target.value)
          }
          placeholder={
            "e.g., python 'print(\"done\")' or echo 'Recording finished'"
          }
          variant="compact"
          disabled={isUpdating("on_recording_end_script")}
          className="w-full"
        />
      </SettingContainer>
    );
  });
