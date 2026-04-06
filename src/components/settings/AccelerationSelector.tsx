import React, { useEffect, useState, useMemo } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useSettings } from "../../hooks/useSettings";
import {
  WhisperAcceleratorSetting,
  OrtAcceleratorSetting,
  AvailableAccelerators,
} from "../../lib/types";
import { Dropdown } from "../ui/Dropdown";
import { SettingContainer } from "../ui/SettingContainer";

interface AccelerationSelectorProps {
  descriptionMode?: "tooltip" | "inline";
  grouped?: boolean;
}

const labelMap: Record<string, string> = {
  auto: "Auto (Default)",
  cpu: "CPU",
  gpu: "GPU (CUDA)",
  cuda: "CUDA",
  directml: "DirectML",
  rocm: "ROCm",
};

export const AccelerationSelector: React.FC<AccelerationSelectorProps> = ({
  descriptionMode = "inline",
  grouped = false,
}) => {
  const { getSetting, updateSetting } = useSettings();
  const [availableAccelerators, setAvailableAccelerators] =
    useState<AvailableAccelerators | null>(null);
  const [isLoadingAccelerators, setIsLoadingAccelerators] = useState(true);

  const currentWhisperAccel = getSetting("whisper_accelerator") ?? "auto";
  const currentOrtAccel = getSetting("ort_accelerator") ?? "auto";
  const currentGpuDevice = getSetting("whisper_gpu_device") ?? -1;

  useEffect(() => {
    const loadAccelerators = async () => {
      try {
        const accelerators = await invoke<AvailableAccelerators>(
          "get_available_accelerators",
        );
        setAvailableAccelerators(accelerators);
      } catch (error) {
        console.error("Failed to load available accelerators:", error);
      } finally {
        setIsLoadingAccelerators(false);
      }
    };
    loadAccelerators();
  }, []);

  const whisperOptions = useMemo(() => {
    const options = availableAccelerators?.whisper ?? ["auto", "cpu", "gpu"];
    return options.map((value) => ({
      value: value as WhisperAcceleratorSetting,
      label: labelMap[value] ?? value,
    }));
  }, [availableAccelerators]);

  const ortOptions = useMemo(() => {
    const options = availableAccelerators?.ort ?? [
      "auto",
      "cpu",
      "cuda",
      "directml",
      "rocm",
    ];
    return options.map((value) => ({
      value: value as OrtAcceleratorSetting,
      label: labelMap[value] ?? value,
    }));
  }, [availableAccelerators]);

  const gpuDeviceOptions = useMemo(() => {
    const options = [{ value: "-1", label: "Default Device" }];
    if (availableAccelerators?.gpu_devices) {
      for (const device of availableAccelerators.gpu_devices) {
        const vram = device.total_vram_mb
          ? ` (${Math.round(device.total_vram_mb / 1024)}GB)`
          : "";
        options.push({
          value: device.id.toString(),
          label: `${device.name}${vram}`,
        });
      }
    }
    return options;
  }, [availableAccelerators]);

  const handleWhisperChange = async (
    event: React.ChangeEvent<HTMLSelectElement>,
  ) => {
    const newValue = event.target.value as WhisperAcceleratorSetting;
    try {
      await updateSetting("whisper_accelerator", newValue);
    } catch (error) {
      console.error("Failed to update whisper accelerator:", error);
    }
  };

  const handleOrtChange = async (
    event: React.ChangeEvent<HTMLSelectElement>,
  ) => {
    const newValue = event.target.value as OrtAcceleratorSetting;
    try {
      await updateSetting("ort_accelerator", newValue);
    } catch (error) {
      console.error("Failed to update ORT accelerator:", error);
    }
  };

  const handleGpuDeviceChange = async (
    event: React.ChangeEvent<HTMLSelectElement>,
  ) => {
    const newValue = parseInt(event.target.value, 10);
    try {
      await updateSetting("whisper_gpu_device", newValue);
    } catch (error) {
      console.error("Failed to update GPU device:", error);
    }
  };

  const isLoading = isLoadingAccelerators;

  return (
    <SettingContainer
      title="Acceleration"
      description="Configure transcription engine acceleration and GPU device"
      descriptionMode={descriptionMode}
      grouped={grouped}
    >
      <div className="space-y-3">
        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--text-secondary)]">
            Whisper Accelerator
          </label>
          <Dropdown
            options={whisperOptions}
            selectedValue={currentWhisperAccel}
            onSelect={(value) =>
              handleWhisperChange({
                target: { value },
              } as React.ChangeEvent<HTMLSelectElement>)
            }
            disabled={isLoading}
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--text-secondary)]">
            ONNX Runtime Accelerator
          </label>
          <Dropdown
            options={ortOptions}
            selectedValue={currentOrtAccel}
            onSelect={(value) =>
              handleOrtChange({
                target: { value },
              } as React.ChangeEvent<HTMLSelectElement>)
            }
            disabled={isLoading}
          />
        </div>

        <div className="flex flex-col gap-1">
          <label className="text-xs text-[var(--text-secondary)]">
            GPU Device
          </label>
          <Dropdown
            options={gpuDeviceOptions}
            selectedValue={currentGpuDevice.toString()}
            onSelect={(value) =>
              handleGpuDeviceChange({
                target: { value },
              } as React.ChangeEvent<HTMLSelectElement>)
            }
            disabled={isLoading}
          />
        </div>
      </div>
    </SettingContainer>
  );
};
