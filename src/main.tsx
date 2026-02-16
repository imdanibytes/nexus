import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./i18n";
import { initTheme } from "./lib/theme";
import "./index.css";
import App from "./App";

initTheme();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
