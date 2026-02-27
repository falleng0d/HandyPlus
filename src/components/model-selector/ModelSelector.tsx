import React, { useEffect, useRef, useState } from "react";
import ModelStatusButton from "./ModelStatusButton";
import ModelDropdown from "./ModelDropdown";
import DownloadProgressDisplay from "./DownloadProgressDisplay";
import { useModelsContext } from "../../contexts/ModelsContext.tsx";

interface ModelSelectorProps {
  onError?: (error: string) => void;
}

const ModelSelector: React.FC<ModelSelectorProps> = ({ onError }) => {
  const {
    models,
    currentModel,
    modelStatus,
    modelError,
    downloadProgress,
    downloadStats,
    selectModel,
    downloadModel,
    deleteModel,
    getModelDisplayText,
  } = useModelsContext();

  const [showModelDropdown, setShowModelDropdown] = useState(false);
  const dropdownRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handleClickOutside = (event: MouseEvent) => {
      if (
        dropdownRef.current &&
        !dropdownRef.current.contains(event.target as Node)
      ) {
        setShowModelDropdown(false);
      }
    };

    document.addEventListener("mousedown", handleClickOutside);
    return () => document.removeEventListener("mousedown", handleClickOutside);
  }, []);

  const handleModelSelect = async (modelId: string) => {
    setShowModelDropdown(false);
    const ok = await selectModel(modelId);
    if (!ok) {
      onError?.(modelError ?? "Failed to switch model");
    }
  };

  const handleModelDownload = async (modelId: string) => {
    const ok = await downloadModel(modelId, { activateAfterDownload: true });
    if (!ok) {
      onError?.(modelError ?? "Failed to download model");
    }
  };

  const handleModelDelete = async (modelId: string) => {
    const ok = await deleteModel(modelId);
    if (!ok) {
      onError?.(modelError ?? "Failed to delete model");
    }
  };

  return (
    <>
      <div className="relative" ref={dropdownRef}>
        <ModelStatusButton
          status={modelStatus}
          displayText={getModelDisplayText()}
          isDropdownOpen={showModelDropdown}
          onClick={() => setShowModelDropdown(!showModelDropdown)}
        />

        {showModelDropdown && (
          <ModelDropdown
            models={models}
            currentModelId={currentModel}
            downloadProgress={downloadProgress}
            onModelSelect={handleModelSelect}
            onModelDownload={handleModelDownload}
            onModelDelete={handleModelDelete}
            onError={onError}
          />
        )}
      </div>

      <DownloadProgressDisplay
        downloadProgress={downloadProgress}
        downloadStats={downloadStats}
      />
    </>
  );
};

export default ModelSelector;
