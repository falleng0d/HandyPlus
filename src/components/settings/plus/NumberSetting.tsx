import React, { useEffect, useState } from "react";
import { Input } from "../../ui/Input";
import { SettingContainer } from "../../ui/SettingContainer";

interface NumberSettingProps {
  title: string;
  description: string;
  value: number;
  min: number;
  max: number;
  step: number;
  unit?: string;
  disabled?: boolean;
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
  onCommit: (value: number) => void | Promise<void>;
}

/** Number input that saves on blur or Enter, so typing doesn't save every digit. */
export const NumberSetting: React.FC<NumberSettingProps> = ({
  title,
  description,
  value,
  min,
  max,
  step,
  unit,
  disabled = false,
  descriptionMode = "tooltip",
  grouped = false,
  onCommit,
}) => {
  const [draft, setDraft] = useState(String(value));

  useEffect(() => {
    setDraft(String(value));
  }, [value]);

  const commit = () => {
    const parsed = Number(draft);
    if (draft.trim() === "" || Number.isNaN(parsed)) {
      setDraft(String(value));
      return;
    }
    const clamped = Math.min(max, Math.max(min, parsed));
    setDraft(String(clamped));
    if (clamped !== value) {
      void onCommit(clamped);
    }
  };

  return (
    <SettingContainer
      title={title}
      description={description}
      descriptionMode={descriptionMode}
      grouped={grouped}
      disabled={disabled}
    >
      <div className="flex items-center gap-2">
        <Input
          type="number"
          variant="compact"
          className="w-24"
          min={min}
          max={max}
          step={step}
          value={draft}
          disabled={disabled}
          onChange={(event) => setDraft(event.target.value)}
          onBlur={commit}
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.currentTarget.blur();
            }
          }}
        />
        {unit && <span className="text-sm text-mid-gray">{unit}</span>}
      </div>
    </SettingContainer>
  );
};
