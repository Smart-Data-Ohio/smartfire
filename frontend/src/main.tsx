import { lazy, StrictMode, Suspense } from "react";
import { createRoot } from "react-dom/client";
import { AppRoot } from "./features/shell/app-root.tsx";
import { restoreAppearance } from "./lib/appearance.ts";
import "./styles/index.css";

restoreAppearance();

// A minimal route switch until the router lands (TanStack Router replaces this). The kitchen sink
// is its own chunk, so none of the design-system demo code ships in the entry.
const KitchenSink = lazy(() => import("./routes/kitchen-sink/kitchen-sink.tsx"));

function Routes() {
  if (window.location.pathname.startsWith(`${import.meta.env.BASE_URL}_kitchen-sink`)) {
    return (
      <Suspense fallback={null}>
        <KitchenSink />
      </Suspense>
    );
  }

  return <AppRoot />;
}

const container = document.getElementById("root");

if (container === null) {
  throw new Error("index.html has no #root element");
}

createRoot(container).render(
  <StrictMode>
    <Routes />
  </StrictMode>,
);
