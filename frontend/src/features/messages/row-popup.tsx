import { useEffect, useLayoutEffect, useRef } from "react";
import type { Placement } from "../../lib/anchor.ts";
import { EmojiImage } from "../../lib/emoji/emoji-image.tsx";
import { LazyEmojiPicker } from "../../lib/emoji/lazy-emoji-picker.tsx";
import { type EmojiChoice, quickReactions, useRecentEmoji } from "../../lib/emoji/recent.ts";
import { readDurationMs } from "../../motion/durations.ts";
import type { MessageDTO } from "../../store/model.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { Menu, MenuQuickItem, MenuQuickRow, type MenuTriggerProps } from "../../ui/menu.tsx";
import { Popover, type PopoverTriggerProps } from "../../ui/popover.tsx";
import { BoostForm } from "./boost-form.tsx";
import { loadCustomIcons } from "./commands.ts";
import { type MenuCommand, MessageMenuItems } from "./message-menu.tsx";
import type { MessagePermissions } from "./permissions.ts";

/** Where a row's popup hangs: a box in the row's own coordinates, and the side to open on. */
export interface PopupOrigin {
  readonly x: number;
  readonly y: number;
  readonly width: number;
  readonly height: number;
  readonly placement: Placement;
}

export type PopupKind = "menu" | "picker" | "boost";

export interface PopupRequest {
  /** Bumps per request, so a closing popup can't clear the one that replaced it. */
  readonly id: number;
  readonly kind: PopupKind;
  readonly origin: PopupOrigin;
  /** Opened from the hover bar's "More" button (which then shows pressed). */
  readonly fromBar: boolean;
  /** Opened from the keyboard: a menu then lands on its first item, as a key-opened menu does. */
  readonly keyboard: boolean;
}

/** The box of `element` (a hover-bar button, a pill) in `row`'s coordinates. */
export function originFromElement(
  element: Element,
  row: Element,
  placement: Placement,
): PopupOrigin {
  const box = element.getBoundingClientRect();
  const frame = row.getBoundingClientRect();

  return {
    x: box.left - frame.left,
    y: box.top - frame.top,
    width: box.width,
    height: box.height,
    placement,
  };
}

/** A pointer position (right click, long press) in `row`'s coordinates. */
export function originFromPoint(clientX: number, clientY: number, row: Element): PopupOrigin {
  const frame = row.getBoundingClientRect();

  return {
    x: clientX - frame.left,
    y: clientY - frame.top,
    width: 1,
    height: 1,
    placement: "bottom-start",
  };
}

/** For keyboard use: under the row's top-right corner, where the hover bar sits. */
export function originForKeyboard(row: Element): PopupOrigin {
  const frame = row.getBoundingClientRect();

  return {
    x: Math.max(0, frame.width - 48),
    y: 4,
    width: 28,
    height: 28,
    placement: "bottom-end",
  };
}

type TriggerProps = MenuTriggerProps | PopoverTriggerProps;

interface GhostProps {
  readonly trigger: TriggerProps;
  readonly origin: PopupOrigin;
  readonly keyboard: boolean;
  /** The popup finished closing (its exit has played). */
  readonly onClosed: () => void;
}

/**
 * An invisible trigger placed where the popup should hang. The design-system Menu and Popover
 * open from a trigger; this one clicks itself on mount, so a right click, a key or a hover-bar
 * button can open them anywhere in the row.
 */
function Ghost({ trigger, origin, keyboard, onClosed }: GhostProps) {
  const clicked = useRef(false);
  const seenOpen = useRef(false);
  const onClosedRef = useRef(onClosed);
  const expanded = trigger["aria-expanded"];
  const { ref, ...rest } = trigger;

  useLayoutEffect(() => {
    onClosedRef.current = onClosed;
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: opens once, at mount
  useLayoutEffect(() => {
    if (!clicked.current) {
      clicked.current = true;
      // A click's `detail` says how it came: 0 for a key, 1 for a pointer. The menu lands on its
      // first item only for a key, so a right click opens it unhighlighted.
      ref.current?.dispatchEvent(
        new MouseEvent("click", { bubbles: true, cancelable: true, detail: keyboard ? 0 : 1 }),
      );
    }
  }, []);

  useEffect(() => {
    if (expanded) {
      seenOpen.current = true;

      return;
    }

    if (!seenOpen.current) {
      return;
    }

    const timer = window.setTimeout(
      () => onClosedRef.current(),
      readDurationMs("--duration-small-exit"),
    );

    return () => window.clearTimeout(timer);
  }, [expanded]);

  return (
    <button
      {...rest}
      ref={ref}
      type="button"
      tabIndex={-1}
      aria-hidden="true"
      className="message-popup-anchor"
      style={{ left: origin.x, top: origin.y, width: origin.width, height: origin.height }}
    />
  );
}

interface QuickReactionsProps {
  readonly onReact: (choice: EmojiChoice) => void;
  readonly onMore: () => void;
}

/**
 * The message sheet's header on a touch phone, as in Slack and Discord: six reactions (the recent
 * picks, topped up with the server's defaults) and one that opens the full picker.
 */
function QuickReactions({ onReact, onMore }: QuickReactionsProps) {
  const recent = useRecentEmoji();

  return (
    <MenuQuickRow label="Quick reactions">
      {quickReactions(recent, 6).map((choice) => (
        <MenuQuickItem
          key={choice.content}
          label={`React with ${choice.title}`}
          onSelect={() => onReact(choice)}
        >
          {choice.imageUrl === null ? (
            <span aria-hidden="true">{choice.content}</span>
          ) : (
            <EmojiImage src={choice.imageUrl} still={choice.stillUrl} width={24} height={24} />
          )}
        </MenuQuickItem>
      ))}
      <MenuQuickItem label="More reactions" onSelect={onMore}>
        <Icon name="smile-plus" size={20} />
      </MenuQuickItem>
    </MenuQuickRow>
  );
}

interface RowPopupProps {
  readonly request: PopupRequest;
  readonly message: MessageDTO;
  readonly permissions: MessagePermissions;
  readonly saved: boolean;
  readonly inThread: boolean;
  readonly onCommand: (command: MenuCommand) => void;
  readonly onReact: (choice: EmojiChoice) => void;
  readonly onClosed: (id: number) => void;
}

/** The row's open popup: its menu, the emoji picker, or the boost box. One at a time. */
export function RowPopup({
  request,
  message,
  permissions,
  saved,
  inThread,
  onCommand,
  onReact,
  onClosed,
}: RowPopupProps) {
  const closed = () => onClosed(request.id);
  const { origin } = request;

  switch (request.kind) {
    case "menu":
      return (
        <Menu
          label="Message actions"
          placement={origin.placement}
          sheetHeader={
            permissions.react ? (
              <QuickReactions onReact={onReact} onMore={() => onCommand("react")} />
            ) : undefined
          }
          trigger={(trigger) => (
            <Ghost
              trigger={trigger}
              origin={origin}
              keyboard={request.keyboard}
              onClosed={closed}
            />
          )}
        >
          <MessageMenuItems
            message={message}
            permissions={permissions}
            saved={saved}
            inThread={inThread}
            onCommand={onCommand}
          />
        </Menu>
      );
    case "picker":
      return (
        <Popover
          label="Add a reaction"
          placement={origin.placement}
          trigger={(trigger) => (
            <Ghost
              trigger={trigger}
              origin={origin}
              keyboard={request.keyboard}
              onClosed={closed}
            />
          )}
        >
          {(close) => (
            <LazyEmojiPicker
              loadCustomIcons={loadCustomIcons}
              onPick={(choice) => {
                onReact(choice);
                close();
              }}
            />
          )}
        </Popover>
      );
    case "boost":
      return (
        <Popover
          label="Boost this message"
          placement={origin.placement}
          trigger={(trigger) => (
            <Ghost
              trigger={trigger}
              origin={origin}
              keyboard={request.keyboard}
              onClosed={closed}
            />
          )}
        >
          {(close) => <BoostForm message={message} onDone={close} />}
        </Popover>
      );
  }
}
