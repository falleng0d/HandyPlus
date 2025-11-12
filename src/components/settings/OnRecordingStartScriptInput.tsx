import React, { useEffect, useState } from "react";
import { SettingContainer, Textarea } from "../ui";
import { useSettings } from "../../hooks/useSettings";

interface OnRecordingStartScriptInputProps {
  descriptionMode?: "inline" | "tooltip";
  grouped?: boolean;
}

export const OnRecordingStartScriptInput: React.FC<OnRecordingStartScriptInputProps> =
  React.memo(({ descriptionMode = "tooltip", grouped = false }) => {
    const { getSetting, updateSetting, isUpdating } = useSettings();
    const [draft, setDraft] = useState("");

    const script = getSetting("on_recording_start_script") ?? "";

    useEffect(() => {
      setDraft(script);
    }, [script]);

    return (
      <SettingContainer
        title="On Recording Start Script"
        description={`Execute a one-liner script when recording starts. Examples: python 'print("aaa")' or echo 'Recording started'`}
        descriptionMode={descriptionMode}
        grouped={grouped}
      >
        <Textarea
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onBlur={(e) =>
            updateSetting("on_recording_start_script", e.target.value)
          }
          placeholder="e.g., python 'print(\"
          variant="compact"
          disabled={isUpdating("on_recording_start_script")}
          className="w-full"
        />
      </SettingContainer>
    );
  });
