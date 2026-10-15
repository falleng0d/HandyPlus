import React, { useEffect, useState } from "react";
import { Input } from "../../ui/Input";
import { SettingContainer } from "../../ui/SettingContainer";

interface TextSettingProps {
  title: string;
  description: string;
  value: string;
  placeholder?: string;
  disabled?: boolean;
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
  onCommit: (value: string) => void | Promise<void>;
}

/** Text input that saves on blur or Enter. */
export const TextSetting: React.FC<TextSettingProps> = ({
  title,
  description,
  value,
  placeholder,
  disabled = false,
  descriptionMode = "tooltip",
  grouped = false,
  onCommit,
}) => {
  const [draft, setDraft] = useState(value);

  useEffect(() => {
    setDraft(value);
  }, [value]);

  return (
    <SettingContainer
      title={title}
      description={description}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
      layout="stacked"
    >
      <Input
        type="text"
        variant="compact"
        className="w-full font-mono"
        value={draft}
        placeholder={placeholder}
        disabled={disabled}
        onChange={(event) => setDraft(event.target.value)}
        onBlur={() => {
          if (draft !== value) {
            void onCommit(draft);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            event.currentTarget.blur();
          }
        }}
      />
    </SettingContainer>
  );
};
