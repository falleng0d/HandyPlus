import React, { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { commands } from "@/bindings";
import { formatKeyCombination } from "../../../../lib/utils/keyboard";
import { useOsType } from "../../../../hooks/useOsType";

interface HandyKeysEvent {
  modifiers: string[];
  key: string | null;
  is_key_down: boolean;
  hotkey_string: string;
}

interface ShortcutRecorderProps {
  value: string;
  onChange: (shortcut: string) => Promise<void> | void;
  disabled?: boolean;
}

/** Records a shortcut with handy-keys and hands it to `onChange`. */
export const ShortcutRecorder: React.FC<ShortcutRecorderProps> = ({
  value,
  onChange,
  disabled = false,
}) => {
  const { t } = useTranslation();
  const osType = useOsType();
  const [isRecording, setIsRecording] = useState(false);
  const [currentKeys, setCurrentKeys] = useState("");
  const containerRef = useRef<HTMLDivElement | null>(null);
  const keyedShortcutRef = useRef("");
  const modifierOnlyShortcutRef = useRef("");
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!isRecording) return;

    let active = true;
    let disposed = false;
    let unlisten: (() => void) | undefined;

    const finish = async (keys: string) => {
      active = false;
      try {
        await onChangeRef.current(keys);
      } finally {
        setIsRecording(false);
      }
    };

    listen<HandyKeysEvent>("handy-keys-event", (event) => {
      if (!active) return;
      const { hotkey_string, is_key_down, key, modifiers } = event.payload;

      if (is_key_down && hotkey_string) {
        if (key) {
          keyedShortcutRef.current = hotkey_string;
        } else {
          modifierOnlyShortcutRef.current = hotkey_string;
        }
        setCurrentKeys(hotkey_string);
      } else if (!is_key_down && key) {
        const keys = keyedShortcutRef.current || hotkey_string;
        if (keys) void finish(keys);
      } else if (
        !is_key_down &&
        !key &&
        modifiers.length === 0 &&
        !keyedShortcutRef.current &&
        modifierOnlyShortcutRef.current
      ) {
        void finish(modifierOnlyShortcutRef.current);
      }
    }).then((stopListening) => {
      if (disposed) {
        stopListening();
      } else {
        unlisten = stopListening;
      }
    });

    return () => {
      disposed = true;
      active = false;
      unlisten?.();
      commands.stopHandyKeysRecording().catch(console.error);
    };
  }, [isRecording]);

  useEffect(() => {
    if (!isRecording) return;

    const handleClickOutside = (e: MouseEvent) => {
      if (
        containerRef.current &&
        !containerRef.current.contains(e.target as Node)
      ) {
        setIsRecording(false);
      }
    };

    window.addEventListener("click", handleClickOutside);
    return () => window.removeEventListener("click", handleClickOutside);
  }, [isRecording]);

  const startRecording = async () => {
    if (isRecording || disabled) return;
    const result = await commands.startHandyKeysRecording("language");
    if (result.status === "error") {
      toast.error(
        t("settings.languages.errors.record", { error: String(result.error) }),
      );
      return;
    }
    keyedShortcutRef.current = "";
    modifierOnlyShortcutRef.current = "";
    setCurrentKeys("");
    setIsRecording(true);
  };

  return (
    <div ref={containerRef} className="flex items-center">
      {isRecording ? (
        <div className="px-2 py-1 text-sm font-semibold whitespace-nowrap border border-logo-primary bg-logo-primary/30 rounded-md">
          {currentKeys
            ? formatKeyCombination(currentKeys, osType)
            : t("settings.general.shortcut.pressKeys")}
        </div>
      ) : (
        <button
          type="button"
          disabled={disabled}
          onClick={startRecording}
          className="px-2 py-1 text-sm font-semibold whitespace-nowrap bg-mid-gray/10 border border-mid-gray/80 rounded-md cursor-pointer hover:bg-logo-primary/10 hover:border-logo-primary disabled:opacity-50 disabled:cursor-not-allowed"
        >
          {value
            ? formatKeyCombination(value, osType)
            : t("settings.languages.noShortcut")}
        </button>
      )}
    </div>
  );
};
