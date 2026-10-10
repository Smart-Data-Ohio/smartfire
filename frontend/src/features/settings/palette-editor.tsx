import { type KeyboardEvent, useEffect, useId, useMemo, useRef, useState } from "react";
import {
  baseTokenColours,
  endCustomTokenPreview,
  previewCustomTokens,
  useAppearance,
  useResolvedTheme,
} from "../../lib/appearance.ts";
import {
  type ColourReader,
  type ContrastResult,
  type CustomTokens,
  contrastReport,
  formatRatio,
  normalizeHex,
  PALETTE_FIELD_GROUPS,
  PALETTE_FIELDS,
  type PaletteField,
  presetColours,
  resetField,
  sameTokens,
  setFieldColour,
  themeColours,
  toHex,
  tokensPayload,
} from "../../lib/custom-palette.ts";
import { PALETTES } from "../../lib/palette.ts";
import { Button } from "../../ui/button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";

/** Every token the editor reads from the page. */
const EDITOR_TOKENS = PALETTE_FIELDS.map((field) => field.token);

const FULL_HEX = /^#?[\da-f]{6}$/i;

const THEME_NAMES = { light: "light", dark: "dark" } as const;

/**
 * Settings → Appearance → Custom palette: the chosen preset's main colours, each changeable, shown
 * across the app as they change (through `previewCustomTokens`) and saved to the account in one
 * `tokens` write. Contrast below the recommended ratios is pointed out, never refused.
 */
export function PaletteEditor({
  disabled,
  onSave,
}: {
  readonly disabled: boolean;
  readonly onSave: (tokens: CustomTokens | null) => Promise<void>;
}) {
  const { palette, customTokens } = useAppearance();
  const theme = useResolvedTheme();
  // This editor's claim on the preview, so a save finishing after it closed leaves the next alone.
  const [owner] = useState(() => Symbol("palette editor"));
  // Colours being tried out; null while the editor shows the account's own.
  const [draft, setDraft] = useState<CustomTokens | null>(null);
  const latest = useRef<CustomTokens | null>(null);
  const tokens = draft ?? customTokens;

  // What each token starts from in both themes, as the page computes it (workspace CSS included).
  const bases = useMemo(() => {
    const computed = baseTokenColours(EDITOR_TOKENS);
    const read: ColourReader = (token, scheme) => computed[scheme].get(token) ?? null;

    return {
      light: presetColours(palette, "light", read),
      dark: presetColours(palette, "dark", read),
    };
  }, [palette]);

  const colours = useMemo(() => themeColours(tokens, bases), [tokens, bases]);
  const presetLabel = PALETTES.find((choice) => choice.value === palette)?.label ?? "the preset";
  const dirty = draft !== null && !sameTokens(draft, customTokens);
  const customised = Object.keys(tokens).length > 0;

  // Leaving the page drops whatever wasn't saved.
  useEffect(() => () => endCustomTokenPreview(owner), [owner]);

  const edit = (next: CustomTokens) => {
    latest.current = next;
    setDraft(next);
    previewCustomTokens(next, owner);
  };

  const discard = () => {
    latest.current = null;
    setDraft(null);
    endCustomTokenPreview(owner);
  };

  const save = () => {
    if (draft === null) return;

    const sent = draft;

    // Edits made while the save was in flight stay on screen, unsaved.
    void onSave(tokensPayload(sent)).then(
      () => {
        if (latest.current === sent) discard();
      },
      () => undefined,
    );
  };

  return (
    <div className="palette-editor">
      <p className="settings-hint text-faint">
        Starts from {presetLabel}. Changes show across Smartfire as you edit, and apply in light and
        dark alike once saved.
      </p>
      <PalettePreview />
      {PALETTE_FIELD_GROUPS.map((group) => (
        <fieldset key={group.title} className="palette-editor-group">
          <legend className="palette-editor-group-title">{group.title}</legend>
          {group.fields.map((field) => (
            <ColourField
              key={field.token}
              field={field}
              colour={toHex(colours[theme].get(field.token) ?? { r: 0, g: 0, b: 0 })}
              custom={tokens[field.token] !== undefined}
              disabled={disabled}
              onChange={(colour) => edit(setFieldColour(tokens, field, colour))}
              onReset={() => edit(resetField(tokens, field))}
            />
          ))}
        </fieldset>
      ))}
      <ContrastList results={contrastReport(colours)} />
      <div className="palette-editor-actions">
        <Button
          variant="primary"
          disabled={!dirty}
          loading={disabled && dirty}
          loadingLabel="Saving"
          onClick={save}
        >
          Save colours
        </Button>
        <Button variant="ghost" disabled={!dirty || disabled} onClick={discard}>
          Discard changes
        </Button>
        <Button
          variant="secondary"
          icon="rotate-ccw"
          disabled={!customised || disabled}
          onClick={() => edit({})}
        >
          Reset to {presetLabel}
        </Button>
        {dirty ? (
          <span className="palette-editor-status text-muted" role="status">
            Unsaved changes
          </span>
        ) : null}
      </div>
    </div>
  );
}

/** One colour: a native picker, its hex for typing or pasting, and a reset to the preset's. */
function ColourField({
  field,
  colour,
  custom,
  disabled,
  onChange,
  onReset,
}: {
  readonly field: PaletteField;
  readonly colour: string;
  readonly custom: boolean;
  readonly disabled: boolean;
  readonly onChange: (colour: string) => void;
  readonly onReset: () => void;
}) {
  const id = useId();
  const [text, setText] = useState(colour);
  const [shown, setShown] = useState(colour);
  const [invalid, setInvalid] = useState(false);

  // A colour from elsewhere (the picker, a reset, another device) replaces what was typed.
  if (shown !== colour) {
    setShown(colour);
    setText(colour);
    setInvalid(false);
  }

  const take = (hex: string) => {
    // Already shown as typed: the colour coming back mustn't rewrite the field.
    setShown(hex);

    if (hex !== colour) onChange(hex);
  };

  /** On leaving the field or Enter: a short hex is expanded, anything incomplete is pointed out. */
  const settle = () => {
    const hex = normalizeHex(text);

    if (hex === null) {
      setInvalid(true);

      return;
    }

    setText(hex);
    take(hex);
  };

  return (
    <div className="palette-field" data-custom={custom || undefined}>
      <input
        id={id}
        type="color"
        className="palette-field-swatch"
        value={colour}
        disabled={disabled}
        aria-describedby={`${id}-hint`}
        onChange={(event) => onChange(event.target.value)}
      />
      <span className="palette-field-text">
        <label htmlFor={id} className="palette-field-label">
          {field.label}
        </label>
        <span id={`${id}-hint`} className="palette-field-hint text-faint">
          {field.hint}
          {custom ? <span className="palette-field-custom"> · Custom colour</span> : null}
        </span>
      </span>
      <span className="palette-field-hex">
        <input
          className={`input palette-field-hex-input${invalid ? " is-error" : ""}`}
          value={text}
          disabled={disabled}
          spellCheck={false}
          autoComplete="off"
          autoCapitalize="off"
          aria-label={`${field.label} hex value`}
          aria-invalid={invalid || undefined}
          aria-describedby={invalid ? `${id}-error` : undefined}
          onChange={(event) => {
            const typed = event.target.value;

            setText(typed);
            setInvalid(false);

            // Only a whole six-digit colour shows while typing: "#123" may be the start of more.
            if (FULL_HEX.test(typed.trim())) take(normalizeHex(typed) ?? colour);
          }}
          onBlur={settle}
          onKeyDown={(event: KeyboardEvent<HTMLInputElement>) => {
            if (event.key !== "Enter") return;

            event.preventDefault();
            settle();
          }}
        />
        {invalid ? (
          <span id={`${id}-error`} className="settings-error" role="alert">
            Use a six- or three-digit hex colour, such as #4f46e5.
          </span>
        ) : null}
      </span>
      <Button
        variant="ghost"
        size="sm"
        className="palette-field-reset"
        disabled={!custom || disabled}
        aria-label={`Reset ${field.label} to the preset`}
        onClick={onReset}
      >
        Reset
      </Button>
    </div>
  );
}

/** "Meets 4.5:1", or where it falls short: "Below 4.5:1 in dark", "… in both themes". */
function verdict(result: ContrastResult): string {
  const [only, other] = result.failing;

  if (only === undefined) return `Meets ${result.minimum}:1`;

  return `Below ${result.minimum}:1 ${other === undefined ? `in ${THEME_NAMES[only]}` : "in both themes"}`;
}

/** Each text and UI pair's WCAG 2 ratio, with the ones under AA called out in words. */
function ContrastList({ results }: { readonly results: readonly ContrastResult[] }) {
  const titleId = useId();
  const low = results.filter((result) => !result.passes).length;

  return (
    <section className="palette-contrast" aria-labelledby={titleId}>
      <h3 id={titleId} className="palette-editor-group-title">
        Contrast
      </h3>
      <p className="settings-hint" aria-live="polite" data-low={low > 0 || undefined}>
        {low === 0
          ? "Every pair meets WCAG AA."
          : `${low} of ${results.length} pairs are below WCAG AA and may be hard to read. You can still save.`}
      </p>
      <ul className="palette-contrast-list">
        {results.map((result) => (
          <li
            key={`${result.foreground} ${result.background}`}
            className="palette-contrast-row"
            data-low={!result.passes || undefined}
          >
            <span className="palette-contrast-name">{result.label}</span>
            <span className="palette-contrast-ratio">{formatRatio(result.ratio)}</span>
            <span className="palette-contrast-verdict">
              {result.passes ? null : <Icon name="alert" size={14} />}
              {verdict(result)}
            </span>
          </li>
        ))}
      </ul>
    </section>
  );
}

/** A sidebar and a few messages in the colours on screen: selection, a mention and a button. */
function PalettePreview() {
  return (
    <figure
      className="palette-preview"
      aria-label="Preview: a selected conversation, a mention and buttons in these colours"
    >
      <span className="palette-preview-sidebar" aria-hidden>
        <span className="palette-preview-row"># general</span>
        <span className="palette-preview-row" data-selected>
          # launch-planning
        </span>
        <span className="palette-preview-row"># design</span>
      </span>
      <span className="palette-preview-pane" aria-hidden>
        <span className="palette-preview-message">
          <span className="palette-preview-name">Maya Okafor</span>
          <span className="palette-preview-time">10:42 AM</span>
          <span className="palette-preview-text">
            Notes are in <span className="palette-preview-link">the brief</span>.
          </span>
        </span>
        <span className="palette-preview-message palette-preview-mention">
          <span className="palette-preview-name">Sam Lee</span>
          <span className="palette-preview-time">10:44 AM</span>
          <span className="palette-preview-text">
            <span className="palette-preview-at">@Riel</span> can you sign off today?
          </span>
        </span>
        <span className="palette-preview-actions">
          <span className="palette-preview-button">Approve</span>
          <span className="palette-preview-danger">Decline</span>
        </span>
      </span>
    </figure>
  );
}
