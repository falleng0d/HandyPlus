import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ModelInfo } from "../lib/types";
import { useSettingsStore } from "../stores/settingsStore";

export type ModelStatus =
  | "ready"
  | "loading"
  | "downloading"
  | "extracting"
  | "error"
  | "unloaded"
  | "none";

interface ModelStateEvent {
  event_type: string;
  model_id?: string;
  error?: string;
}

interface DownloadProgress {
  model_id: string;
  downloaded: number;
  total: number;
  percentage: number;
}

interface DownloadStats {
  startTime: number;
  lastUpdate: number;
  totalDownloaded: number;
  speed: number;
}

interface DownloadModelOptions {
  activateAfterDownload?: boolean;
}

interface ModelsContextValue {
  models: ModelInfo[];
  currentModel: string;
  modelStatus: ModelStatus;
  modelError: string | null;
  loading: boolean;
  downloadingModels: Set<string>;
  extractingModels: Set<string>;
  downloadProgress: Map<string, DownloadProgress>;
  downloadStats: Map<string, DownloadStats>;
  hasAnyModels: boolean;
  isFirstRun: boolean;
  loadModels: () => Promise<void>;
  loadCurrentModel: () => Promise<void>;
  checkFirstRun: () => Promise<boolean>;
  selectModel: (modelId: string) => Promise<boolean>;
  downloadModel: (
    modelId: string,
    options?: DownloadModelOptions,
  ) => Promise<boolean>;
  deleteModel: (modelId: string) => Promise<boolean>;
  getModelInfo: (modelId: string) => ModelInfo | undefined;
  isModelDownloading: (modelId: string) => boolean;
  isModelExtracting: (modelId: string) => boolean;
  getDownloadProgress: (modelId: string) => DownloadProgress | undefined;
  getModelDisplayText: () => string;
}

const ModelsContext = createContext<ModelsContextValue | null>(null);

export const ModelsProvider: React.FC<{ children: React.ReactNode }> = ({
  children,
}) => {
  const setSettings = useSettingsStore((state) => state.setSettings);

  const [models, setModels] = useState<ModelInfo[]>([]);
  const [currentModel, setCurrentModel] = useState<string>("");
  const [modelStatus, setModelStatus] = useState<ModelStatus>("unloaded");
  const [modelError, setModelError] = useState<string | null>(null);
  const [downloadingModels, setDownloadingModels] = useState<Set<string>>(
    new Set(),
  );
  const [extractingModels, setExtractingModels] = useState<Set<string>>(
    new Set(),
  );
  const [downloadProgress, setDownloadProgress] = useState<
    Map<string, DownloadProgress>
  >(new Map());
  const [downloadStats, setDownloadStats] = useState<
    Map<string, DownloadStats>
  >(new Map());
  const [loading, setLoading] = useState(true);
  const [hasAnyModels, setHasAnyModels] = useState(false);
  const [isFirstRun, setIsFirstRun] = useState(false);

  const activateAfterDownloadRef = useRef<Set<string>>(new Set());

  const loadModels = useCallback(async () => {
    try {
      const modelList = await invoke<ModelInfo[]>("get_available_models");
      setModels(modelList);
      setModelError(null);
    } catch (err) {
      setModelError(`Failed to load models: ${err}`);
    } finally {
      setLoading(false);
    }
  }, []);

  const loadCurrentModel = useCallback(async () => {
    try {
      const current = await invoke<string>("get_current_model");
      setCurrentModel(current);

      const currentSettings = useSettingsStore.getState().settings;
      setSettings(
        currentSettings
          ? {
              ...currentSettings,
              selected_model: current,
            }
          : currentSettings,
      );

      if (current) {
        const transcriptionStatus = await invoke<string | null>(
          "get_transcription_model_status",
        );
        if (transcriptionStatus === current) {
          setModelStatus("ready");
        } else {
          setModelStatus("unloaded");
        }
      } else {
        setModelStatus("none");
      }
    } catch (err) {
      console.error("Failed to load current model:", err);
      setModelStatus("error");
      setModelError("Failed to check model status");
    }
  }, [setSettings]);

  const checkFirstRun = useCallback(async () => {
    try {
      const hasModels = await invoke<boolean>("has_any_models_available");
      setHasAnyModels(hasModels);
      setIsFirstRun(!hasModels);
      return !hasModels;
    } catch (err) {
      console.error("Failed to check model availability:", err);
      return false;
    }
  }, []);

  const selectModel = useCallback(
    async (modelId: string) => {
      try {
        setModelError(null);
        await invoke("set_active_model", { modelId });
        setCurrentModel(modelId);

        const currentSettings = useSettingsStore.getState().settings;
        setSettings(
          currentSettings
            ? {
                ...currentSettings,
                selected_model: modelId,
              }
            : currentSettings,
        );

        setIsFirstRun(false);
        setHasAnyModels(true);
        return true;
      } catch (err) {
        setModelError(`Failed to switch to model: ${err}`);
        setModelStatus("error");
        return false;
      }
    },
    [setSettings],
  );

  const downloadModel = useCallback(
    async (modelId: string, options?: DownloadModelOptions) => {
      const activateAfterDownload = options?.activateAfterDownload === true;
      const knownModel = models.find((model) => model.id === modelId);

      if (knownModel?.is_downloaded) {
        if (activateAfterDownload) {
          return selectModel(modelId);
        }
        return true;
      }

      if (knownModel?.is_downloading) {
        if (activateAfterDownload) {
          activateAfterDownloadRef.current.add(modelId);
        }
        return true;
      }

      try {
        setModelError(null);

        if (activateAfterDownload) {
          activateAfterDownloadRef.current.add(modelId);
        }

        setDownloadingModels((prev) => {
          const next = new Set(prev);
          next.add(modelId);
          return next;
        });

        await invoke("download_model", { modelId });
        return true;
      } catch (err) {
        setModelError(`Failed to download model: ${err}`);
        activateAfterDownloadRef.current.delete(modelId);
        setDownloadingModels((prev) => {
          const next = new Set(prev);
          next.delete(modelId);
          return next;
        });
        return false;
      }
    },
    [models, selectModel],
  );

  const deleteModel = useCallback(
    async (modelId: string) => {
      try {
        setModelError(null);
        await invoke("delete_model", { modelId });
        await loadModels();
        return true;
      } catch (err) {
        setModelError(`Failed to delete model: ${err}`);
        return false;
      }
    },
    [loadModels],
  );

  const getModelInfo = useCallback(
    (modelId: string): ModelInfo | undefined => {
      return models.find((model) => model.id === modelId);
    },
    [models],
  );

  const isModelDownloading = useCallback(
    (modelId: string): boolean => {
      return downloadingModels.has(modelId);
    },
    [downloadingModels],
  );

  const isModelExtracting = useCallback(
    (modelId: string): boolean => {
      return extractingModels.has(modelId);
    },
    [extractingModels],
  );

  const getDownloadProgress = useCallback(
    (modelId: string): DownloadProgress | undefined => {
      return downloadProgress.get(modelId);
    },
    [downloadProgress],
  );

  const getModelDisplayText = useCallback((): string => {
    if (extractingModels.size > 0) {
      if (extractingModels.size === 1) {
        const [modelId] = Array.from(extractingModels);
        const model = models.find((m) => m.id === modelId);
        return `Extracting ${model?.name || "Model"}...`;
      }
      return `Extracting ${extractingModels.size} models...`;
    }

    if (downloadProgress.size > 0) {
      if (downloadProgress.size === 1) {
        const [progress] = Array.from(downloadProgress.values());
        const percentage = Math.max(
          0,
          Math.min(100, Math.round(progress.percentage)),
        );
        return `Downloading ${percentage}%`;
      }
      return `Downloading ${downloadProgress.size} models...`;
    }

    const active = models.find((m) => m.id === currentModel);

    switch (modelStatus) {
      case "ready":
        return active?.name || "Model Ready";
      case "loading":
        return active ? `Loading ${active.name}...` : "Loading...";
      case "extracting":
        return active ? `Extracting ${active.name}...` : "Extracting...";
      case "error":
        return modelError || "Model Error";
      case "unloaded":
        return active?.name || "Model Unloaded";
      case "none":
        return "No Model - Download Required";
      default:
        return active?.name || "Model Unloaded";
    }
  }, [
    currentModel,
    downloadProgress,
    extractingModels,
    modelError,
    modelStatus,
    models,
  ]);

  const updateModelAndSettings = useCallback(
    (modelId: string) => {
      setCurrentModel(modelId);
      const currentSettings = useSettingsStore.getState().settings;
      setSettings(
        currentSettings
          ? {
              ...currentSettings,
              selected_model: modelId,
            }
          : currentSettings,
      );
    },
    [setSettings],
  );

  useEffect(() => {
    void loadModels();
    void loadCurrentModel();
    void checkFirstRun();

    const modelStateUnlisten = listen<ModelStateEvent>(
      "model-state-changed",
      (event) => {
        const { event_type, model_id, error } = event.payload;

        switch (event_type) {
          case "loading_started":
            setModelStatus("loading");
            setModelError(null);
            if (model_id) {
              updateModelAndSettings(model_id);
            }
            break;
          case "loading_completed":
            setModelStatus("ready");
            setModelError(null);
            if (model_id) {
              updateModelAndSettings(model_id);
            }
            break;
          case "loading_failed":
            setModelStatus("error");
            setModelError(error || "Failed to load model");
            break;
          case "unloaded":
            setModelStatus("unloaded");
            setModelError(null);
            break;
        }
      },
    );

    const progressUnlisten = listen<DownloadProgress>(
      "model-download-progress",
      (event) => {
        const progress = event.payload;

        setDownloadProgress((prev) => {
          const next = new Map(prev);
          next.set(progress.model_id, progress);
          return next;
        });
        setModelStatus("downloading");

        const now = Date.now();
        setDownloadStats((prev) => {
          const current = prev.get(progress.model_id);
          const next = new Map(prev);

          if (!current) {
            next.set(progress.model_id, {
              startTime: now,
              lastUpdate: now,
              totalDownloaded: progress.downloaded,
              speed: 0,
            });
          } else {
            const timeDiff = (now - current.lastUpdate) / 1000;
            const bytesDiff = progress.downloaded - current.totalDownloaded;

            if (timeDiff > 0.5) {
              const currentSpeed = bytesDiff / (1024 * 1024) / timeDiff;
              const validCurrentSpeed = Math.max(0, currentSpeed);
              const smoothedSpeed =
                current.speed > 0
                  ? current.speed * 0.8 + validCurrentSpeed * 0.2
                  : validCurrentSpeed;

              next.set(progress.model_id, {
                startTime: current.startTime,
                lastUpdate: now,
                totalDownloaded: progress.downloaded,
                speed: Math.max(0, smoothedSpeed),
              });
            }
          }

          return next;
        });
      },
    );

    const completeUnlisten = listen<string>(
      "model-download-complete",
      (event) => {
        const modelId = event.payload;

        setDownloadingModels((prev) => {
          const next = new Set(prev);
          next.delete(modelId);
          return next;
        });
        setDownloadProgress((prev) => {
          const next = new Map(prev);
          next.delete(modelId);
          return next;
        });
        setDownloadStats((prev) => {
          const next = new Map(prev);
          next.delete(modelId);
          return next;
        });

        void loadModels();
        setHasAnyModels(true);
        setIsFirstRun(false);

        if (activateAfterDownloadRef.current.has(modelId)) {
          activateAfterDownloadRef.current.delete(modelId);
          void selectModel(modelId);
        } else {
          void loadCurrentModel();
        }
      },
    );

    const extractionStartedUnlisten = listen<string>(
      "model-extraction-started",
      (event) => {
        const modelId = event.payload;
        setExtractingModels((prev) => {
          const next = new Set(prev);
          next.add(modelId);
          return next;
        });
        setModelStatus("extracting");
      },
    );

    const extractionCompletedUnlisten = listen<string>(
      "model-extraction-completed",
      (event) => {
        const modelId = event.payload;
        setExtractingModels((prev) => {
          const next = new Set(prev);
          next.delete(modelId);
          return next;
        });
        void loadModels();
      },
    );

    const extractionFailedUnlisten = listen<{
      model_id: string;
      error: string;
    }>("model-extraction-failed", (event) => {
      const modelId = event.payload.model_id;
      setExtractingModels((prev) => {
        const next = new Set(prev);
        next.delete(modelId);
        return next;
      });
      activateAfterDownloadRef.current.delete(modelId);
      setModelError(`Failed to extract model: ${event.payload.error}`);
      setModelStatus("error");
    });

    return () => {
      modelStateUnlisten.then((fn) => fn());
      progressUnlisten.then((fn) => fn());
      completeUnlisten.then((fn) => fn());
      extractionStartedUnlisten.then((fn) => fn());
      extractionCompletedUnlisten.then((fn) => fn());
      extractionFailedUnlisten.then((fn) => fn());
    };
  }, [checkFirstRun, loadCurrentModel, loadModels, selectModel, setSettings, updateModelAndSettings]);

  const value = useMemo<ModelsContextValue>(
    () => ({
      models,
      currentModel,
      modelStatus,
      modelError,
      loading,
      downloadingModels,
      extractingModels,
      downloadProgress,
      downloadStats,
      hasAnyModels,
      isFirstRun,
      loadModels,
      loadCurrentModel,
      checkFirstRun,
      selectModel,
      downloadModel,
      deleteModel,
      getModelInfo,
      isModelDownloading,
      isModelExtracting,
      getDownloadProgress,
      getModelDisplayText,
    }),
    [
      models,
      currentModel,
      modelStatus,
      modelError,
      loading,
      downloadingModels,
      extractingModels,
      downloadProgress,
      downloadStats,
      hasAnyModels,
      isFirstRun,
      loadModels,
      loadCurrentModel,
      checkFirstRun,
      selectModel,
      downloadModel,
      deleteModel,
      getModelInfo,
      isModelDownloading,
      isModelExtracting,
      getDownloadProgress,
      getModelDisplayText,
    ],
  );

  return (
    <ModelsContext.Provider value={value}>{children}</ModelsContext.Provider>
  );
};

export const useModelsContext = (): ModelsContextValue => {
  const context = useContext(ModelsContext);
  if (!context) {
    throw new Error("useModels must be used within a ModelsProvider");
  }
  return context;
};

export type { DownloadProgress, DownloadStats, DownloadModelOptions };
