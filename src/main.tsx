import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { ModelsProvider } from "./contexts/ModelsContext";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <ModelsProvider>
      <App />
    </ModelsProvider>
  </React.StrictMode>,
);
