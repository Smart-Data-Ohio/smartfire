import { createContext, type ReactNode, use, useCallback, useId, useState } from "react";
import type { Settings } from "../../gen/Settings.ts";
import { ActionError } from "../../sync/run.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { toast } from "../../ui/toast-store.ts";
import type { Choice } from "./settings-format.ts";

/** The loaded settings page and the way a section hands back the server's answer to a write. */
export interface SettingsState {
  readonly settings: Settings;
  readonly replace: (next: Settings) => void;
  /**
   * Changes the page from whatever it holds when the change lands, so answers that settle out of
   * order each touch only their own part.
   */
  readonly update: (change: (current: Settings) => Settings) => void;
}

export const SettingsContext = createContext<SettingsState | null>(null);

/** The settings page, for a section inside the settings view. */
export function useSettings(): SettingsState {
  const state = use(SettingsContext);

  if (state === null) {
    throw new Error("useSettings needs the settings view");
  }

  return state;
}

/** A failed write as a field map (empty unless the server named fields). */
export function fieldsOf(error: Error): Readonly<Record<string, readonly string[]>> {
  return error instanceof ActionError ? error.fields : {};
}

/** A section's writes in flight, by control. */
export interface Busy {
  /** Whether any of `keys` is saving. */
  readonly busy: (...keys: readonly string[]) => boolean;
  /** Marks `key` saving until `work` settles, and hands `work` back. */
  readonly track: <T>(key: string, work: Promise<T>) => Promise<T>;
}

/**
 * Tracks each control's write on its own, so one settling never re-enables a control whose own
 * write is still in flight.
 */
export function useBusy(): Busy {
  const [saving, setSaving] = useState<ReadonlySet<string>>(() => new Set());

  const track = useCallback(<T,>(key: string, work: Promise<T>): Promise<T> => {
    setSaving((keys) => new Set(keys).add(key));

    return work.finally(() =>
      setSaving((keys) => {
        const next = new Set(keys);

        next.delete(key);

        return next;
      }),
    );
  }, []);

  return { busy: (...keys) => keys.some((key) => saving.has(key)), track };
}

/** Tells the person a write failed, with the server's reason. */
export function toastFailure(title: string, error: Error): void {
  toast({ title, description: error.message, tone: "danger" });
}

/** A section's page: its title, one line on what lives here, and its groups. */
export function SettingsPage({
  title,
  description,
  children,
}: {
  readonly title: string;
  readonly description?: ReactNode;
  readonly children: ReactNode;
}) {
  return (
    <section className="settings-page enter-fade" aria-labelledby="settings-page-title">
      <header className="settings-page-header">
        <h1 id="settings-page-title" className="text-page">
          {title}
        </h1>
        {description === undefined ? null : (
          <p className="settings-page-description text-muted">{description}</p>
        )}
      </header>
      {children}
    </section>
  );
}

/** One group of settings, the classic page's fieldset: a heading, a hint, the controls. */
export function SettingsGroup({
  title,
  description,
  children,
  id,
}: {
  readonly title: string;
  readonly description?: ReactNode;
  readonly children: ReactNode;
  readonly id?: string;
}) {
  const generated = useId();
  const headingId = `${id ?? generated}-title`;

  return (
    <section className="settings-group" aria-labelledby={headingId} id={id}>
      <h2 id={headingId} className="settings-group-title text-title">
        {title}
      </h2>
      {description === undefined ? null : (
        <p className="settings-group-description text-muted">{description}</p>
      )}
      <div className="settings-group-body">{children}</div>
    </section>
  );
}

/** A labelled native select: the platform picker on phones, typeahead on desktops. */
export function SettingsSelect<T extends string>({
  label,
  value,
  choices,
  onChange,
  disabled,
  hint,
}: {
  readonly label: string;
  readonly value: T;
  readonly choices: readonly Choice<T>[];
  readonly onChange: (value: T) => void;
  readonly disabled?: boolean;
  readonly hint?: string;
}) {
  const id = useId();

  return (
    <div className="settings-field">
      <label htmlFor={id} className="settings-label">
        {label}
      </label>
      <select
        id={id}
        className="input settings-select"
        value={value}
        disabled={disabled}
        aria-describedby={hint === undefined ? undefined : `${id}-hint`}
        onChange={(event) => {
          const picked = choices.find((choice) => choice.value === event.target.value);

          if (picked !== undefined) {
            onChange(picked.value);
          }
        }}
      >
        {choices.map((choice) => (
          <option key={choice.value} value={choice.value}>
            {choice.label}
          </option>
        ))}
      </select>
      {hint === undefined ? null : (
        <p id={`${id}-hint`} className="settings-hint text-faint">
          {hint}
        </p>
      )}
    </div>
  );
}

/** A row of mutually exclusive choices (WAI-ARIA radio group, native radios underneath). */
export function SettingsRadios<T extends string>({
  label,
  value,
  choices,
  onChange,
  disabled,
}: {
  readonly label: string;
  readonly value: T;
  readonly choices: readonly Choice<T>[];
  readonly onChange: (value: T) => void;
  readonly disabled?: boolean;
}) {
  const name = useId();

  return (
    <fieldset className="settings-radios" disabled={disabled}>
      <legend className="settings-label">{label}</legend>
      <div className="settings-radio-row">
        {choices.map((choice) => (
          <label key={choice.value} className="settings-radio">
            <input
              type="radio"
              name={name}
              value={choice.value}
              checked={value === choice.value}
              onChange={() => onChange(choice.value)}
            />
            <span>{choice.label}</span>
          </label>
        ))}
      </div>
    </fieldset>
  );
}

/** The classic page's error line under a control, announced when it appears. */
export function FieldError({ message }: { readonly message: string | undefined }) {
  return message === undefined ? null : (
    <p className="settings-error" role="alert">
      {message}
    </p>
  );
}

/** A link to the classic page, which loads in full. */
export function ClassicLink({
  href,
  children,
}: {
  readonly href: string;
  readonly children: string;
}) {
  return (
    <a className="settings-classic-link" href={href}>
      {children}
      <Icon name="external-link" size={14} />
    </a>
  );
}
