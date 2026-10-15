import { toast } from "sonner";
import {
  commands,
  type AppSettings as Settings,
  type Result,
} from "@/bindings";

/** Rejects so the store rolls the optimistic value back, and tells the user why. */
async function unwrapOrToast(pending: Promise<Result<null, string>>) {
  const result = await pending;
  if (result.status === "error") {
    toast.error(result.error);
    throw new Error(result.error);
  }
}

export const plusSettingUpdaters: {
  [K in keyof Settings]?: (value: Settings[K]) => Promise<unknown>;
} = {
  vad_threshold: (value) =>
    unwrapOrToast(commands.changeVadThresholdSetting(value as number)),
  vad_prefill_ms: (value) =>
    unwrapOrToast(commands.changeVadPrefillMsSetting(value as number)),
  vad_hangover_ms: (value) =>
    unwrapOrToast(commands.changeVadHangoverMsSetting(value as number)),
  vad_onset_ms: (value) =>
    unwrapOrToast(commands.changeVadOnsetMsSetting(value as number)),
  context_lines: (value) =>
    unwrapOrToast(commands.changeContextLinesSetting(value as number)),
  context_max_characters: (value) =>
    unwrapOrToast(
      commands.changeContextMaxCharactersSetting(value as number | null),
    ),
};
