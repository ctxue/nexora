import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import "@fontsource-variable/inter";
import "@fontsource-variable/geist";

import App from "./App";
import { applyInitialAppearance } from "./features/settings/appearance";
import SettingsWindow from "./SettingsWindow";
import "./styles/index.css";

const isSettingsWindow = getCurrentWindow().label === "settings";

applyInitialAppearance();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    {isSettingsWindow ? <SettingsWindow /> : <App />}
  </React.StrictMode>,
);
