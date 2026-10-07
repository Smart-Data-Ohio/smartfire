import { type CSSProperties, useId } from "react";
import {
  type FontPreset,
  type PalettePreset,
  setFont,
  setPalette,
  useAppearance,
} from "../../lib/appearance.ts";
import { DEFAULT_PALETTE_TOKENS, PALETTES, paletteTokens } from "../../lib/palette.ts";

interface FontChoice {
  readonly value: FontPreset;
  readonly label: string;
}

const FONT_CHOICES: readonly FontChoice[] = [
  { value: "inter", label: "Inter" },
  { value: "system", label: "System" },
  { value: "atkinson", label: "Atkinson Hyperlegible" },
  { value: "serif", label: "Source Serif" },
  { value: "mono", label: "JetBrains Mono" },
];

/**
 * A palette's tokens as inline custom properties, so its swatch paints in it whatever palette the
 * page is in (Smartfire's swatch takes the stylesheet's own values back).
 */
function swatchStyle(palette: PalettePreset): CSSProperties {
  const tokens = palette === "smartfire" ? DEFAULT_PALETTE_TOKENS : paletteTokens(palette);

  return Object.fromEntries(tokens);
}

/** The colour palettes, each a radio drawn as a small picture of the app in its colours. */
export function PalettePicker() {
  const { palette } = useAppearance();
  const name = useId();
  const legend = useId();

  return (
    <fieldset className="settings-radios" role="radiogroup" aria-labelledby={legend}>
      <legend id={legend} className="settings-label">
        Colour palette
      </legend>
      <div className="palette-swatches">
        {PALETTES.map((choice) => (
          <label key={choice.value} className="palette-swatch">
            <input
              type="radio"
              className="visually-hidden"
              name={name}
              value={choice.value}
              checked={palette === choice.value}
              onChange={() => setPalette(choice.value)}
            />
            <span className="palette-swatch-art" style={swatchStyle(choice.value)} aria-hidden>
              <span className="palette-swatch-rail" />
              <span className="palette-swatch-pane">
                <span className="palette-swatch-line" />
                <span className="palette-swatch-line palette-swatch-line-short" />
                <span className="palette-swatch-button" />
              </span>
            </span>
            <span className="palette-swatch-name">{choice.label}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** The fonts, each label set in its own face. */
export function FontPicker() {
  const { font } = useAppearance();
  const name = useId();
  const legend = useId();

  return (
    <fieldset className="settings-radios" role="radiogroup" aria-labelledby={legend}>
      <legend id={legend} className="settings-label">
        Font
      </legend>
      <div className="settings-radio-row">
        {FONT_CHOICES.map((choice) => (
          <label key={choice.value} className="settings-radio">
            <input
              type="radio"
              name={name}
              value={choice.value}
              checked={font === choice.value}
              onChange={() => setFont(choice.value)}
            />
            <span style={{ fontFamily: `var(--font-preset-${choice.value})` }}>{choice.label}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** A message as the timeline would show it, in this device's palette and font. */
export function AppearancePreview() {
  return (
    <figure className="appearance-preview" aria-label="Preview">
      <span className="appearance-preview-avatar" aria-hidden>
        MO
      </span>
      <div className="appearance-preview-body">
        <p className="appearance-preview-meta">
          <span className="appearance-preview-name">Maya Okafor</span>
          <span className="appearance-preview-time">10:42 AM</span>
        </p>
        <p className="appearance-preview-text">
          The launch notes are up in <span className="appearance-preview-link">#launch-planning</span>
          , thanks <span className="appearance-preview-mention">@Riel</span> for the review.
        </p>
      </div>
    </figure>
  );
}
