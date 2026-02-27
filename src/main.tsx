import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ModelsProvider } from "./contexts/ModelsContext";
import { SelectedLanguageProvider } from "./contexts/SelectedLanguageContext";
import { SelectedPromptProvider } from "./contexts/SelectedPromptContext";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ModelsProvider>
      <SelectedLanguageProvider>
        <SelectedPromptProvider>
          <App />
        </SelectedPromptProvider>
      </SelectedLanguageProvider>
    </ModelsProvider>
  </React.StrictMode>,
);
