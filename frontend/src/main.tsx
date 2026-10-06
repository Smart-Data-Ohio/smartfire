import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { AppRoot } from "./features/shell/app-root.tsx";

const container = document.getElementById("root");

if (container === null) {
  throw new Error("index.html has no #root element");
}

createRoot(container).render(
  <StrictMode>
    <AppRoot />
  </StrictMode>,
);
