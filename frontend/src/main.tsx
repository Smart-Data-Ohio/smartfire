// The global stylesheet first: it declares the cascade-layer order (tokens.css), and every
// component stylesheet imported after it lands in its `app` or `ui` layer in that order.
import "./styles/index.css";
import { RouterProvider } from "@tanstack/react-router";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { restoreAppearance } from "./lib/appearance.ts";
import { router } from "./router.tsx";
import { followAccountAppearance } from "./sync/settings.ts";

// Before the first render: the account's theme and text size from the inline boot JSON, under any
// theme pinned on this device. Then boot and `/me` keep the account's choices on screen.
restoreAppearance();

followAccountAppearance();

const container = document.getElementById("root");

if (container === null) {
  throw new Error("index.html has no #root element");
}

createRoot(container).render(
  <StrictMode>
    <RouterProvider router={router} />
  </StrictMode>,
);
