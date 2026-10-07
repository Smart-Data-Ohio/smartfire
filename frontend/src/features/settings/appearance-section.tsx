import { useState } from "react";
import type { TextSize } from "../../gen/TextSize.ts";
import type { Theme } from "../../gen/Theme.ts";
import type { UpdateAppearance } from "../../gen/UpdateAppearance.ts";
import {
  type DensityPreference,
  type MotionPreference,
  setDensity,
  setMotion,
  setTheme,
  useAppearance,
} from "../../lib/appearance.ts";
import { settings as settingsActions } from "../../sync/settings.ts";
import { type Choice, TEXT_SIZE_CHOICES, THEME_CHOICES } from "./settings-format.ts";
import {
  SettingsGroup,
  SettingsPage,
  SettingsRadios,
  SettingsSelect,
  toastFailure,
  useSettings,
} from "./settings-parts.tsx";

const DENSITY_CHOICES: readonly Choice<DensityPreference>[] = [
  { value: "comfortable", label: "Comfortable" },
  { value: "compact", label: "Compact" },
];

const MOTION_CHOICES: readonly Choice<MotionPreference>[] = [
  { value: "system", label: "Match my system" },
  { value: "reduce", label: "Reduce motion" },
  { value: "full", label: "Full motion" },
];

/** `<html data-text-size>`, as the server renders it from the account's choice. */
function applyTextSize(size: TextSize): void {
  document.documentElement.dataset.textSize = size;
}

/**
 * Appearance: the account's theme, text size and time zone (saved for every device, as the
 * classic page saves them), then this device's own density and motion. Choosing a theme shows it
 * at once here too.
 */
export function AppearanceSection() {
  const { settings, replace } = useSettings();
  const { appearance } = settings;
  const device = useAppearance();
  const [busy, setBusy] = useState<string | null>(null);
  // The choice shows at once; the server's answer replaces it (or a failure drops it).
  const [chosen, setChosen] = useState<Partial<UpdateAppearance>>({});

  const save = (key: string, change: Partial<UpdateAppearance>) => {
    setBusy(key);
    setChosen((held) => ({ ...held, ...change }));
    settingsActions
      .updateAppearance(change)
      .then(replace, (error: Error) => toastFailure("Couldn't save your appearance", error))
      .finally(() => {
        setBusy(null);
        setChosen({});
      });
  };

  const zoneChoices: readonly Choice<string>[] = [
    { value: "", label: "Not set (use system)" },
    ...appearance.timeZones,
  ];

  return (
    <SettingsPage title="Appearance" description="How Smartfire looks and keeps time for you.">
      <SettingsGroup title="Theme and text" description="Saved to your account, for every device.">
        <SettingsRadios
          label="Theme"
          value={chosen.theme ?? appearance.theme}
          choices={THEME_CHOICES}
          disabled={busy === "theme"}
          onChange={(theme: Theme) => {
            setTheme(theme);
            save("theme", { theme });
          }}
        />
        <SettingsRadios
          label="Text size"
          value={chosen.textSize ?? appearance.textSize}
          choices={TEXT_SIZE_CHOICES}
          disabled={busy === "text"}
          onChange={(textSize: TextSize) => {
            applyTextSize(textSize);
            save("text", { textSize });
          }}
        />
        <SettingsSelect
          label="Time zone"
          value={chosen.timeZone ?? appearance.timeZone ?? ""}
          choices={zoneChoices}
          disabled={busy === "zone"}
          hint="Detected from your browser on first visit. Times, reminders and quiet hours use it."
          onChange={(timeZone) => save("zone", { timeZone })}
        />
      </SettingsGroup>

      <SettingsGroup title="This device" description="Only this browser remembers these.">
        <SettingsRadios
          label="Density"
          value={device.density}
          choices={DENSITY_CHOICES}
          onChange={setDensity}
        />
        <SettingsRadios
          label="Motion"
          value={device.motion}
          choices={MOTION_CHOICES}
          onChange={setMotion}
        />
      </SettingsGroup>
    </SettingsPage>
  );
}
