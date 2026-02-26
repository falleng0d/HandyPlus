import React, { useEffect, useRef, useState } from "react";
import { type } from "@tauri-apps/plugin-os";
import {
  formatKeyCombination,
  getKeyName,
  normalizeKey,
  type OSType,
} from "../../lib/utils/keyboard";

interface ShortcutRecorderProps {
  value: string;
  onChange: (shortcut: string) => void;
  placeholder?: string;
  disabled?: boolean;
  /** Called when the user clicks to start recording (e.g. to suspend an existing binding) */
  onSuspend?: () => Promise<void>;
  /** Called after commit or cancel (e.g. to resume the binding) */
  onResume?: () => Promise<void>;
  className?: string;
}

export const ShortcutRecorder: React.FC<ShortcutRecorderProps> = ({
  value,
  onChange,
  placeholder = "Click to record",
  disabled = false,
  onSuspend,
  onResume,
  className,
}) => {
  const [recordedKeys, setRecordedKeys] = useState<string[]>([]);
  const [isRecording, setIsRecording] = useState(false);
  const [originalValue, setOriginalValue] = useState<string>("");
  const [osType, setOsType] = useState<OSType>("unknown");
  const recorderRef = useRef<HTMLDivElement>(null);

  // Refs to avoid stale closures in event handlers
  const keyPressedRef = useRef<string[]>([]);
  const recordedKeysRef = useRef<string[]>([]);

  useEffect(() => {
    const detectOsType = async () => {
      try {
        const detectedType = type();
        let normalizedType: OSType;
        switch (detectedType) {
          case "macos":
            normalizedType = "macos";
            break;
          case "windows":
            normalizedType = "windows";
            break;
          case "linux":
            normalizedType = "linux";
            break;
          default:
            normalizedType = "unknown";
        }
        setOsType(normalizedType);
      } catch {
        setOsType("unknown");
      }
    };
    detectOsType();
  }, []);

  const stopRecording = async (cancelled: boolean) => {
    if (cancelled) {
      onChange(originalValue);
    }
    await onResume?.().catch(console.error);
    setIsRecording(false);
    keyPressedRef.current = [];
    recordedKeysRef.current = [];
    setRecordedKeys([]);
    setOriginalValue("");
  };

  useEffect(() => {
    if (!isRecording) return;

    let cleanup = false;

    const handleKeyDown = (e: KeyboardEvent) => {
      if (cleanup) return;
      if (e.repeat) return;

      if (e.key === "Escape") {
        void stopRecording(true);
        return;
      }

      e.preventDefault();
      const rawKey = getKeyName(e, osType);
      const key = normalizeKey(rawKey);

      if (!keyPressedRef.current.includes(key)) {
        keyPressedRef.current = [...keyPressedRef.current, key];
        if (!recordedKeysRef.current.includes(key)) {
          recordedKeysRef.current = [...recordedKeysRef.current, key];
          setRecordedKeys([...recordedKeysRef.current]);
        }
      }
    };

    const handleKeyUp = async (e: KeyboardEvent) => {
      if (cleanup) return;
      e.preventDefault();

      const rawKey = getKeyName(e, osType);
      const key = normalizeKey(rawKey);

      keyPressedRef.current = keyPressedRef.current.filter((k) => k !== key);

      if (
        keyPressedRef.current.length === 0 &&
        recordedKeysRef.current.length > 0
      ) {
        const newShortcut = recordedKeysRef.current.join("+");
        onChange(newShortcut);
        await onResume?.().catch(console.error);
        if (!cleanup) {
          setIsRecording(false);
          keyPressedRef.current = [];
          recordedKeysRef.current = [];
          setRecordedKeys([]);
          setOriginalValue("");
        }
      }
    };

    const handleClickOutside = (e: MouseEvent) => {
      if (cleanup) return;
      if (
        recorderRef.current &&
        !recorderRef.current.contains(e.target as Node)
      ) {
        void stopRecording(true);
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    window.addEventListener("keyup", handleKeyUp);
    window.addEventListener("click", handleClickOutside);

    return () => {
      cleanup = true;
      window.removeEventListener("keydown", handleKeyDown);
      window.removeEventListener("keyup", handleKeyUp);
      window.removeEventListener("click", handleClickOutside);
    };
  }, [isRecording, osType]);

  const startRecording = async () => {
    if (disabled || isRecording) return;
    await onSuspend?.().catch(console.error);
    setOriginalValue(value);
    keyPressedRef.current = [];
    recordedKeysRef.current = [];
    setRecordedKeys([]);
    setIsRecording(true);
  };

  const formatCurrentKeys = (): string => {
    if (recordedKeys.length === 0) return "Press keys...";
    return formatKeyCombination(recordedKeys.join("+"), osType);
  };

  const displayValue = value
    ? formatKeyCombination(value, osType)
    : placeholder;

  if (isRecording) {
    return (
      <div
        ref={recorderRef}
        className="px-2 py-1 text-sm font-semibold border border-logo-primary bg-logo-primary/30 rounded min-w-[120px] text-center cursor-default select-none"
      >
        {formatCurrentKeys()}
      </div>
    );
  }

  return (
    <div
      ref={recorderRef}
      className={`px-2 py-1 text-sm font-semibold bg-mid-gray/10 border border-mid-gray/80 rounded min-w-[120px] text-center select-none content-center ${
        disabled
          ? "opacity-50 cursor-not-allowed"
          : "hover:bg-logo-primary/10 cursor-pointer hover:border-logo-primary"
      } ${className}`}
      onClick={startRecording}
    >
      {displayValue}
    </div>
  );
};
