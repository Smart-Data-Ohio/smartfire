import { useState } from "react";
import type { Settings } from "../../gen/Settings.ts";
import type { TextSize } from "../../gen/TextSize.ts";
import type { Theme } from "../../gen/Theme.ts";
import type { UpdateAppearance } from "../../gen/UpdateAppearance.ts";
import {
  showTextSize as applyTextSize,
  type DensityPreference,
  type MotionPreference,
  setDensity,
  setMotion,
  setThemeOverride,
  showAccountTheme,
  type ThemePreference,
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
  useBusy,
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

/** "account" follows the account's theme; the others pin one on this device. */
type DeviceTheme = "account" | ThemePreference;

const DEVICE_THEME_CHOICES: readonly Choice<DeviceTheme>[] = [
  { value: "account", label: "Use my account's theme" },
  { value: "system", label: "Match my system" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
];

/**
 * Appearance: the account's theme, text size and time zone (saved for every device, as the
 * classic page saves them), then this device's own theme pin, density and motion. Choosing a
 * theme or size shows it at once here too.
 */
export function AppearanceSection() {
  const { settings, replace } = useSettings();
  const { appearance } = settings;
  const device = useAppearance();
  const { busy, track } = useBusy();
  // A choice shows at once; the server's answer replaces it, and a failure puts back what was.
  const [chosen, setChosen] = useState<Partial<UpdateAppearance>>({});

  /**
   * Saves `change` under `key`. `show` puts a setting on the page: the choice at once, then the
   * saved value, or the stored one again after a failure.
   */
  const save = (
    key: keyof UpdateAppearance,
    change: Partial<UpdateAppearance>,
    show?: (appearance: Settings["appearance"] | Partial<UpdateAppearance>) => void,
  ) => {
    const before = appearance;

    setChosen((held) => ({ ...held, ...change }));
    show?.(change);
    void track(key, settingsActions.updateAppearance(change))
      .then(
        (next) => {
          replace(next);
          show?.(next.appearance);
        },
        (error: Error) => {
          show?.(before);
          toastFailure("Couldn't save your appearance", error);
        },
      )
      .finally(() =>
        setChosen((held) => {
          const { [key]: _settled, ...rest } = held;

          return rest;
        }),
      );
  };

  const showTheme = ({ theme }: { readonly theme?: Theme | null }) => {
    if (theme !== undefined && theme !== null) {
      showAccountTheme(theme);
    }
  };

  const showTextSize = ({ textSize }: { readonly textSize?: TextSize | null }) => {
    if (textSize !== undefined && textSize !== null) {
      applyTextSize(textSize);
    }
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
          disabled={busy("theme")}
          onChange={(theme: Theme) => save("theme", { theme }, showTheme)}
        />
        <SettingsRadios
          label="Text size"
          value={chosen.textSize ?? appearance.textSize}
          choices={TEXT_SIZE_CHOICES}
          disabled={busy("textSize")}
          onChange={(textSize: TextSize) => save("textSize", { textSize }, showTextSize)}
        />
        <SettingsSelect
          label="Time zone"
          value={chosen.timeZone ?? appearance.timeZone ?? ""}
          choices={zoneChoices}
          disabled={busy("timeZone")}
          hint="Detected from your browser on first visit. Times, reminders and quiet hours use it."
          onChange={(timeZone) => save("timeZone", { timeZone })}
        />
      </SettingsGroup>

      <SettingsGroup title="This device" description="Only this browser remembers these.">
        <SettingsSelect
          label="Theme on this device"
          value={device.themeOverride ?? "account"}
          choices={DEVICE_THEME_CHOICES}
          hint="Pin a theme here without changing it on your other devices."
          onChange={(choice: DeviceTheme) => setThemeOverride(choice === "account" ? null : choice)}
        />
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
