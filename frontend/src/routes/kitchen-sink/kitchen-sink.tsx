import { useState } from "react";
import {
  type MotionPreference,
  setDensity,
  setMotion,
  setThemeOverride,
  type ThemePreference,
  useAppearance,
} from "../../lib/appearance.ts";
import { Button } from "../../ui/button.tsx";
import { Toaster } from "../../ui/toast.tsx";
import { Toggle } from "../../ui/toggle.tsx";
import { Gallery } from "./gallery.tsx";
import { MockApp } from "./mock-app.tsx";
import "./kitchen-sink.css";

const THEMES: readonly ThemePreference[] = ["system", "light", "dark"];

const MOTIONS: readonly MotionPreference[] = ["system", "reduce", "full"];

function Choice<T extends string>({
  label,
  options,
  value,
  onChange,
}: {
  readonly label: string;
  readonly options: readonly T[];
  readonly value: T;
  readonly onChange: (value: T) => void;
}) {
  return (
    <fieldset className="ks-choice">
      <legend className="ks-choice-label">{label}</legend>
      {options.map((option) => (
        <Button
          key={option}
          variant="pill"
          size="sm"
          aria-pressed={value === option}
          onClick={() => onChange(option)}
        >
          {option}
        </Button>
      ))}
    </fieldset>
  );
}

/**
 * /app/_kitchen-sink: every design-system component in every variant and state, a miniature of
 * the app, and the appearance switches (theme, density, motion). "Side by side" renders the
 * gallery in light and dark at once: every colour token is a light-dark() pair, so a subtree with
 * its own color-scheme resolves the other theme.
 */
export default function KitchenSink() {
  const appearance = useAppearance();
  const [sideBySide, setSideBySide] = useState(false);

  return (
    <div className="ks">
      <header className="ks-toolbar">
        <div className="ks-title">
          <h1 className="text-title">Smartfire design system</h1>
          <span className="ks-version">v0</span>
        </div>
        <div className="ks-switches">
          <Choice
            label="Theme"
            options={THEMES}
            value={appearance.theme}
            onChange={setThemeOverride}
          />
          <Choice label="Motion" options={MOTIONS} value={appearance.motion} onChange={setMotion} />
          <div className="ks-toggle">
            <Toggle
              checked={appearance.density === "compact"}
              onCheckedChange={(compact) => setDensity(compact ? "compact" : "comfortable")}
              label="Compact"
            />
          </div>
          <div className="ks-toggle">
            <Toggle checked={sideBySide} onCheckedChange={setSideBySide} label="Light and dark" />
          </div>
        </div>
      </header>
      <section className="ks-mock" aria-label="App preview">
        <MockApp />
      </section>
      {sideBySide ? (
        <div className="ks-split">
          <div className="ks-scheme" data-scheme="light">
            <Gallery />
          </div>
          <div className="ks-scheme" data-scheme="dark">
            <Gallery />
          </div>
        </div>
      ) : (
        <Gallery />
      )}
      <Toaster />
    </div>
  );
}
