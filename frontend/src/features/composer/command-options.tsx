import { type KeyboardEvent, useEffect, useId, useLayoutEffect, useRef, useState } from "react";
import type { CommandOption } from "../../gen/CommandOption.ts";
import { useStore } from "../../store/store.ts";
import { Button } from "../../ui/button.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { AutocompleteList } from "./autocomplete/autocomplete-list.tsx";
import {
  choiceSuggestions,
  type MenuKind,
  personSuggestions,
  roomSuggestions,
  type Suggestion,
  selectable,
  subcommandSuggestions,
} from "./autocomplete/suggestions.ts";
import type { Autocomplete } from "./autocomplete/use-autocomplete.ts";
import {
  type CommandEntry,
  chooseSubcommand,
  editOption,
  entryErrors,
  entryOptions,
  hideOption,
  optionChoices,
  optionKind,
  type PickedValue,
  picksValue,
  shownError,
  showOption,
  subcommandsOf,
  touchField,
  typeSubcommand,
} from "./slash.ts";
import "./command-options.css";

/** The subcommand field's key: no option name has a slash in it. */
const SUBCOMMAND = "/subcommand";

/** Focus the first option field once a picked subcommand's fields are on screen. */
const FIRST_OPTION = "/first-option";

/** Person lookups wait this long after the last keystroke, as the `@` menu's do. */
const DEBOUNCE_MS = 120;

/** One field on screen: the subcommand (`option` is `null`) or an option. */
interface Field {
  readonly key: string;
  readonly option: CommandOption | null;
}

interface PeopleResults {
  readonly key: string;
  readonly query: string;
  readonly items: readonly Suggestion[];
}

interface CommandOptionsProps {
  readonly roomId: number;
  readonly entry: CommandEntry;
  /** Applies an edit to the latest entry, so several edits in one event all land. */
  readonly onChange: (edit: (entry: CommandEntry) => CommandEntry) => void;
  /** Enter on the last field: the composer sends, or shows what's wrong. */
  readonly onSubmit: () => void;
  /** Escape, or Backspace before the first field: back to the command as plain text. */
  readonly onCancel: () => void;
  /** Bumped after a refused send: focus goes to the first field showing an error. */
  readonly focusRequest: number;
}

/** What a pick list's row puts in its field. */
function pickOf(item: Suggestion): PickedValue | null {
  switch (item.kind) {
    case "choice":
      return item.choice;
    case "mention":
      return { label: item.user.name, value: item.user.id };
    case "room":
      return { label: `#${item.row.displayName}`, value: item.row.room.id };
    default:
      return null;
  }
}

/** The input mode a phone keyboard opens with: digits only where no minus sign is needed. */
function inputModeFor(option: CommandOption | null): "numeric" | "decimal" | undefined {
  if (option === null || (option.type !== "integer" && option.type !== "number")) {
    return undefined;
  }

  if (option.choices !== undefined || option.min === undefined || option.min < 0) {
    return undefined;
  }

  return option.type === "integer" ? "numeric" : "decimal";
}

/**
 * A typed command's fields, Discord-style, in place of the composer's text: the subcommand
 * (picked from a list), then each option as a labelled field, required ones first, optional ones
 * added from the row below. Choices, yes/no, people and channels pick from a listbox the field
 * drives. Tab or Enter moves to the next field (Enter on the last sends), Backspace in an empty
 * field goes back, Escape turns it all back into plain text. Each field's problem shows under it
 * once the field is left, or after a refused send.
 */
export function CommandOptions({
  roomId,
  entry,
  onChange,
  onSubmit,
  onCancel,
  focusRequest,
}: CommandOptionsProps) {
  const baseId = useId();
  const listboxId = `${baseId}-menu`;
  const inputs = useRef(new Map<string, HTMLInputElement>());
  const subcommands = subcommandsOf(entry.schema);
  const options = entryOptions(entry) ?? [];
  const errors = entryErrors(entry);

  const fields: Field[] = [
    ...(subcommands === null ? [] : [{ key: SUBCOMMAND, option: null }]),
    ...options.flatMap((option) =>
      entry.shown.includes(option.name) ? [{ key: option.name, option }] : [],
    ),
  ];

  const optional = options.filter((option) => !entry.shown.includes(option.name));
  const [focused, setFocused] = useState<string | null>(null);

  const textOf = (field: Field): string =>
    field.option === null ? entry.subcommandText : (entry.inputs[field.key]?.text ?? "");

  /** Where the keyboard starts: the first field in error, else the first empty one, else the last. */
  const startKey = (): string | null => {
    const wrong = fields.find((field) =>
      field.option === null
        ? entry.server.subcommand !== null || (entry.attempted && errors.subcommand !== null)
        : shownError(entry, field.key, errors) !== null,
    );

    const empty = fields.find((field) => textOf(field).trim() === "");

    return (wrong ?? empty ?? fields.at(-1))?.key ?? null;
  };

  const [pendingFocus, setPendingFocus] = useState<string | null>(startKey);
  const [activeKey, setActiveKey] = useState<string | null>(null);
  const [dismissed, setDismissed] = useState<string | null>(null);
  const [people, setPeople] = useState<PeopleResults | null>(null);
  const handledRequest = useRef(focusRequest);
  const focusedField = fields.find((field) => field.key === focused) ?? null;

  /** What the menu filters by: nothing while the field still shows its pick, so all rows show. */
  const queryOf = (field: Field): string => {
    if (field.option === null) {
      return entry.subcommand !== null && entry.subcommandText === entry.subcommand
        ? ""
        : entry.subcommandText;
    }

    const input = entry.inputs[field.key];

    return input?.picked != null && input.text === input.picked.label ? "" : (input?.text ?? "");
  };

  const roomPick = focusedField?.option?.type === "room";
  // Only a channel option reads the sidebar; other fields mustn't rerender on every unread.
  const sidebar = useStore((state) => (roomPick ? state.sidebar : null));
  const personQuery = focusedField?.option?.type === "user" ? queryOf(focusedField) : null;

  useEffect(() => {
    if (personQuery === null || focused === null) {
      return;
    }

    let live = true;

    const timer = window.setTimeout(() => {
      personSuggestions(roomId, personQuery).then(
        (items) => {
          if (live) setPeople({ key: focused, query: personQuery, items });
        },
        () => {
          if (live) setPeople({ key: focused, query: personQuery, items: [] });
        },
      );
    }, DEBOUNCE_MS);

    return () => {
      live = false;
      window.clearTimeout(timer);
    };
  }, [roomId, focused, personQuery]);

  const menuFor = (field: Field): { kind: MenuKind; items: readonly Suggestion[] } | null => {
    const query = queryOf(field);

    if (field.option === null) {
      return { kind: "subcommand", items: subcommandSuggestions(subcommands ?? [], query) };
    }

    const choices = optionChoices(field.option);

    if (choices !== null) {
      return { kind: "choice", items: choiceSuggestions(choices, query) };
    }

    if (field.option.type === "user") {
      return { kind: "mention", items: people?.key === field.key ? people.items : [] };
    }

    if (field.option.type === "room") {
      return { kind: "room", items: sidebar === null ? [] : roomSuggestions(sidebar, query) };
    }

    return null;
  };

  const menu = focusedField === null ? null : menuFor(focusedField);
  const items = menu?.items ?? [];
  const open = menu !== null && items.length > 0 && dismissed !== focused;

  const pickedValue =
    focusedField?.option == null
      ? (entry.subcommand ?? undefined)
      : entry.inputs[focusedField.key]?.picked?.value;

  const activeIndex = (() => {
    const active = items.findIndex((item) => item.key === activeKey);

    if (active >= 0) {
      return active;
    }

    const picked = items.findIndex((item) =>
      item.kind === "subcommand"
        ? item.subcommand.name === pickedValue
        : pickOf(item)?.value === pickedValue,
    );

    return picked >= 0 ? picked : Math.max(0, items.findIndex(selectable));
  })();

  const optionId = (index: number) => `${listboxId}-option-${index}`;

  const autocomplete: Autocomplete = {
    open,
    kind: open ? (menu?.kind ?? null) : null,
    items: open ? items : [],
    activeIndex,
    listboxId,
    optionId,
    setActiveIndex: (index) => {
      const item = items[index];

      if (item !== undefined && selectable(item)) {
        setActiveKey(item.key);
      }
    },
    move: (delta) => {
      const count = items.length;

      if (count > 0) {
        setActiveKey(items[(((activeIndex + delta) % count) + count) % count]?.key ?? null);
      }
    },
    dismiss: () => setDismissed(focused),
  };

  // Focus follows what the keys asked for, once the field it names is on screen; a refused send
  // asks for where the keyboard starts (the first field in error).
  useLayoutEffect(() => {
    if (focusRequest !== handledRequest.current) {
      handledRequest.current = focusRequest;
      setPendingFocus(startKey());

      return;
    }

    if (pendingFocus === null) {
      return;
    }

    const key =
      pendingFocus === FIRST_OPTION
        ? (fields.find((field) => field.option !== null)?.key ?? null)
        : pendingFocus;

    const input = key === null ? undefined : inputs.current.get(key);

    if (key !== null && input === undefined) {
      return;
    }

    setPendingFocus(null);

    if (input !== undefined) {
      input.focus();
      input.setSelectionRange(input.value.length, input.value.length);
    }
  });

  const nextKey = (field: Field, index: number): string | null =>
    field.option === null ? FIRST_OPTION : (fields[index + 1]?.key ?? null);

  const choose = (field: Field, item: Suggestion) => {
    setDismissed(field.key);
    setActiveKey(null);

    if (item.kind === "subcommand") {
      onChange((current) => ({
        ...chooseSubcommand(current, item.subcommand.name),
        subcommandText: item.subcommand.name,
      }));

      return;
    }

    const picked = pickOf(item);

    if (picked !== null) {
      onChange((current) =>
        touchField(editOption(current, field.key, { text: picked.label, picked }), field.key),
      );
    }
  };

  const back = (field: Field, index: number) => {
    if (field.option !== null && !field.option.required) {
      onChange((current) => hideOption(current, field.key));
    }

    const previous = fields[index - 1];

    if (previous === undefined) {
      onCancel();
    } else {
      setPendingFocus(previous.key);
    }
  };

  const onKeyDown = (event: KeyboardEvent<HTMLInputElement>, field: Field, index: number) => {
    if (event.nativeEvent.isComposing) {
      return;
    }

    const plain = !event.metaKey && !event.ctrlKey && !event.altKey;

    if (!plain) {
      return;
    }

    const last = index === fields.length - 1 && field.option !== null;

    if (open && focused === field.key) {
      if (event.key === "ArrowDown" || event.key === "ArrowUp") {
        event.preventDefault();
        autocomplete.move(event.key === "ArrowDown" ? 1 : -1);

        return;
      }

      if ((event.key === "Enter" || event.key === "Tab") && !event.shiftKey) {
        event.preventDefault();

        const item = items[activeIndex];

        if (item !== undefined && selectable(item)) {
          choose(field, item);
        }

        const next = nextKey(field, index);

        if (next !== null) {
          setPendingFocus(next);
        }

        return;
      }

      if (event.key === "Escape") {
        event.preventDefault();
        event.stopPropagation();
        setDismissed(field.key);

        return;
      }
    }

    if (event.key === "Escape") {
      event.preventDefault();
      event.stopPropagation();
      onCancel();

      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      onChange((current) => touchField(current, field.key));

      if (last || fields.length === 1) {
        onSubmit();
      } else {
        setPendingFocus(nextKey(field, index));
      }

      return;
    }

    if (event.key === "Tab" && !event.shiftKey && !last && index < fields.length - 1) {
      event.preventDefault();
      onChange((current) => touchField(current, field.key));
      setPendingFocus(nextKey(field, index));

      return;
    }

    const target = event.currentTarget;

    if (
      event.key === "Backspace" &&
      target.value === "" &&
      target.selectionStart === 0 &&
      target.selectionEnd === 0
    ) {
      event.preventDefault();
      back(field, index);
    }
  };

  const focusedOption = focusedField?.option ?? null;

  const chosen =
    subcommands === null ? null : subcommands.find((sub) => sub.name === entry.subcommand);

  const hint =
    focusedOption !== null
      ? `${focusedOption.description} · ${optionKind(focusedOption)}${focusedOption.required ? "" : " · optional"}`
      : focusedField !== null && chosen === undefined
        ? "Pick a subcommand"
        : (chosen?.description ?? entry.command.description);

  return (
    <fieldset
      className="command-options enter-rise"
      aria-label={`Options for /${entry.command.name}`}
    >
      <AutocompleteList
        autocomplete={autocomplete}
        onPick={(index) => {
          const item = items[index];

          if (focusedField === null || item === undefined || !selectable(item)) {
            return;
          }

          choose(focusedField, item);

          const next = nextKey(focusedField, fields.indexOf(focusedField));

          if (next !== null) {
            setPendingFocus(next);
          }
        }}
      />
      <div className="command-options-head">
        <span className="command-options-name">/{entry.command.name}</span>
        {entry.command.agentName === null ? null : (
          <span className="autocomplete-tag" data-tone="agent">
            {entry.command.agentName}
          </span>
        )}
        <span className="command-options-hint" aria-hidden="true">
          {hint}
        </span>
        <IconButton
          icon="x"
          label="Cancel command"
          shortcut={["Esc"]}
          size="sm"
          className="command-options-cancel"
          onMouseDown={(event) => event.preventDefault()}
          onClick={onCancel}
        />
      </div>
      <div className="command-options-fields">
        {fields.map((field, index) => {
          const { option } = field;
          const id = `${baseId}-field-${index}`;
          const descriptionId = `${id}-description`;
          const errorId = `${id}-error`;
          const pick = option === null || picksValue(option);
          const expanded = open && focused === field.key;

          const error =
            option === null
              ? (entry.server.subcommand ?? (entry.attempted ? errors.subcommand : null))
              : shownError(entry, field.key, errors);

          const description =
            option === null
              ? `Pick one: ${(subcommands ?? []).map((sub) => sub.name).join(", ")}.`
              : `${option.description}. ${optionKind(option)}. ${option.required ? "Required" : "Optional"}.`;

          return (
            <div key={field.key} className="command-field">
              <div className="command-option" data-invalid={error !== null || undefined}>
                <label htmlFor={id} className="command-option-name">
                  {option?.name ?? "subcommand"}
                </label>
                {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: aria-expanded is only set when the role is combobox */}
                <input
                  ref={(node) => {
                    if (node === null) {
                      inputs.current.delete(field.key);
                    } else {
                      inputs.current.set(field.key, node);
                    }
                  }}
                  id={id}
                  className="command-option-input"
                  type="text"
                  value={textOf(field)}
                  autoComplete="off"
                  autoCapitalize="off"
                  spellCheck={false}
                  inputMode={inputModeFor(option)}
                  enterKeyHint={index === fields.length - 1 && option !== null ? "send" : "next"}
                  role={pick ? "combobox" : undefined}
                  aria-autocomplete={pick ? "list" : undefined}
                  aria-expanded={pick ? expanded : undefined}
                  aria-controls={expanded ? listboxId : undefined}
                  aria-activedescendant={expanded ? optionId(activeIndex) : undefined}
                  aria-required={option === null || option.required}
                  aria-invalid={error !== null || undefined}
                  aria-describedby={error === null ? descriptionId : `${descriptionId} ${errorId}`}
                  onChange={(event) => {
                    const text = event.target.value;

                    setDismissed(null);
                    setActiveKey(null);
                    onChange((current) =>
                      option === null
                        ? typeSubcommand(current, text)
                        : editOption(current, field.key, { text, picked: null }),
                    );
                  }}
                  onKeyDown={(event) => onKeyDown(event, field, index)}
                  onFocus={() => {
                    setFocused(field.key);
                    setActiveKey(null);
                  }}
                  onBlur={() => {
                    setFocused((current) => (current === field.key ? null : current));

                    if (option !== null) {
                      onChange((current) =>
                        current.shown.includes(field.key)
                          ? touchField(current, field.key)
                          : current,
                      );
                    }
                  }}
                />
                {option === null || option.required ? null : (
                  <IconButton
                    icon="x"
                    label={`Remove ${option.name}`}
                    size="sm"
                    className="command-option-remove"
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={() => {
                      onChange((current) => hideOption(current, field.key));
                      setPendingFocus(fields[index - 1]?.key ?? fields[index + 1]?.key ?? null);
                    }}
                  />
                )}
              </div>
              <span id={descriptionId} className="visually-hidden">
                {description}
              </span>
              {error === null ? null : (
                <p id={errorId} className="command-option-error">
                  {error}
                </p>
              )}
            </div>
          );
        })}
      </div>
      {optional.length === 0 ? null : (
        <div className="command-options-add">
          <span className="command-options-add-label">Optional</span>
          {optional.map((option) => (
            <Button
              key={option.name}
              variant="ghost"
              size="sm"
              icon="plus"
              className="command-options-add-button"
              aria-label={`Add ${option.name}`}
              title={option.description}
              onMouseDown={(event) => event.preventDefault()}
              onClick={() => {
                onChange((current) => showOption(current, option.name));
                setPendingFocus(option.name);
              }}
            >
              {option.name}
            </Button>
          ))}
        </div>
      )}
      {entry.server.general.length === 0 ? null : (
        <p className="command-options-alert" role="alert">
          {entry.server.general.join(" ")}
        </p>
      )}
    </fieldset>
  );
}
