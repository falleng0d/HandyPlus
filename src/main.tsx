import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ModelsProvider } from "./contexts/ModelsContext";
import { SelectedPromptProvider } from "./contexts/SelectedPromptContext";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ModelsProvider>
      <SelectedPromptProvider>
        <App />
      </SelectedPromptProvider>
    </ModelsProvider>
  </React.StrictMode>,
);
