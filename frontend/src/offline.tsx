// The offline page's entry (offline.html): the page the service worker answers a navigation with
// when the network fails. It loads nothing from the server, so it renders without the boot JSON.
import "./styles/index.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { OfflineScreen } from "./features/offline/offline-screen.tsx";
import { restoreAppearance } from "./lib/appearance.ts";

restoreAppearance();

const container = document.getElementById("root");

if (container === null) {
  throw new Error("offline.html has no #root element");
}

createRoot(container).render(
  <StrictMode>
    <OfflineScreen />
  </StrictMode>,
);
