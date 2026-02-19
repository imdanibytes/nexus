import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "./i18n";
import "./index.css";
import { initColorMode } from "./lib/theme";
import App from "./App";

// Apply saved color mode before first render to avoid flash
initColorMode();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>
);
