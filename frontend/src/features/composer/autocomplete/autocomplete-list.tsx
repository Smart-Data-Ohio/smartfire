import { useLayoutEffect, useState } from "react";
import { usePresence } from "../../../motion/presence.ts";
import { Avatar } from "../../../ui/avatar.tsx";
import { Icon } from "../../../ui/icons/icon.tsx";
import { Kbd } from "../../../ui/kbd.tsx";
import { AgentBadgeFor } from "../../people/agent-badge.tsx";
import { ROOM_KIND_ICON } from "../../room/room-icon.ts";
import { commandUsage } from "../slash.ts";
import { type Suggestion, selectable, TRIGGER_TITLES } from "./suggestions.ts";
import type { TriggerKind } from "./trigger.ts";
import type { Autocomplete } from "./use-autocomplete.ts";
import "./autocomplete.css";

interface Shown {
  readonly kind: TriggerKind;
  readonly items: readonly Suggestion[];
}

interface AutocompleteListProps {
  readonly autocomplete: Autocomplete;
  readonly onPick: (index: number) => void;
}

/**
 * The suggestion menu above the composer: a listbox the textarea drives (it keeps focus and
 * points at the highlighted row with aria-activedescendant). Rows follow the pointer; a click
 * picks without stealing focus. It grows from the composer's corner (menu-dropdown recipe).
 */
export function AutocompleteList({ autocomplete, onPick }: AutocompleteListProps) {
  const { open, items, activeIndex, listboxId, optionId } = autocomplete;
  const presence = usePresence<HTMLDivElement>(open);
  const [shown, setShown] = useState<Shown | null>(null);

  // Keep the last rows on screen while the menu plays its exit.
  if (
    open &&
    autocomplete.kind !== null &&
    (shown?.kind !== autocomplete.kind || shown.items !== items)
  ) {
    setShown({ kind: autocomplete.kind, items });
  }

  const kind = shown?.kind ?? autocomplete.kind ?? "mention";
  const rows = shown?.items ?? items;

  useLayoutEffect(() => {
    if (!open) {
      return;
    }

    document.getElementById(optionId(activeIndex))?.scrollIntoView({ block: "nearest" });
  }, [open, activeIndex, optionId]);

  if (!presence.mounted) {
    return null;
  }

  return (
    <div
      ref={presence.ref}
      className="autocomplete t-dropdown"
      data-state={presence.state}
      data-origin="bottom-left"
      data-kind={kind}
    >
      <div className="autocomplete-head" aria-hidden="true">
        <span className="autocomplete-title">{TRIGGER_TITLES[kind]}</span>
        <span className="autocomplete-keys">
          <Kbd keys={["↑", "↓"]} /> to navigate <Kbd keys={["↵"]} /> to select{" "}
          <Kbd keys={["Esc"]} /> to dismiss
        </span>
      </div>
      <div
        id={listboxId}
        role="listbox"
        aria-label={TRIGGER_TITLES[kind]}
        className="autocomplete-list"
      >
        {rows.map((item, index) => (
          // biome-ignore lint/a11y/useKeyWithClickEvents: the textarea owns the keyboard (aria-activedescendant); a click only picks
          <div
            key={item.key}
            id={optionId(index)}
            role="option"
            tabIndex={-1}
            aria-selected={open && index === activeIndex}
            aria-disabled={!selectable(item) || undefined}
            className="autocomplete-option"
            onPointerMove={() => {
              if (index !== activeIndex) {
                autocomplete.setActiveIndex(index);
              }
            }}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => {
              if (selectable(item)) {
                onPick(index);
              }
            }}
          >
            <SuggestionRow item={item} />
          </div>
        ))}
      </div>
    </div>
  );
}

function SuggestionRow({ item }: { readonly item: Suggestion }) {
  switch (item.kind) {
    case "mention":
      return (
        <>
          <Avatar
            name={item.user.name}
            userId={item.user.id}
            src={item.user.avatarUrl}
            size={22}
            decorative
          />
          <span className="autocomplete-label">{item.user.name}</span>
          <AgentBadgeFor user={item.user} />
          {item.insert === null ? (
            <span className="autocomplete-detail">Duplicate name — type as plain text</span>
          ) : null}
        </>
      );
    case "emoji":
      return (
        <>
          <span className="autocomplete-glyph" aria-hidden="true">
            {item.icon.character ??
              (item.icon.imageUrl === null ? null : (
                <img src={item.icon.imageUrl} alt="" width={20} height={20} />
              ))}
          </span>
          <span className="autocomplete-label">{item.insert}</span>
          <span className="autocomplete-detail">{item.icon.title}</span>
        </>
      );
    case "command":
      return (
        <span className="autocomplete-command">
          <span className="autocomplete-command-line">
            <span className="autocomplete-label">{commandUsage(item.command)}</span>
            {item.command.agentName === null ? null : (
              <span className="autocomplete-tag" data-tone="agent">
                {item.command.agentName}
              </span>
            )}
          </span>
          <span className="autocomplete-detail">{item.command.description}</span>
        </span>
      );
    case "room":
      return (
        <>
          <span className="autocomplete-glyph" aria-hidden="true">
            <Icon name={ROOM_KIND_ICON[item.row.room.kind]} />
          </span>
          <span className="autocomplete-label">{item.row.displayName}</span>
        </>
      );
  }
}
