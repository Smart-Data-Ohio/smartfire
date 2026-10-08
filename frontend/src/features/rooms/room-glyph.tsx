import { useEffect, useState } from "react";
import type { EmojiData } from "../../lib/emoji/data.ts";
import type { RoomKind } from "../../store/model.ts";
import { Icon } from "../../ui/icons/icon.tsx";
import { loadCustomIcons } from "../messages/commands.ts";
import { ROOM_KIND_ICON } from "../room/room-icon.ts";
import { type RoomIconLook, roomIconLook } from "./room-forms.ts";
import "./rooms.css";

const resolved = new Map<string, RoomIconLook>();

/** The emoji catalogue, through the picker's own lazy module, so neither lands in the shell. */
export function loadEmoji(): Promise<EmojiData | null> {
  return import("../../lib/emoji/data.ts")
    .then(
      (module) => module.loadEmojiData(),
      () => null,
    )
    .catch(() => null);
}

/**
 * What `iconName` draws as, as the classic presenters resolve it: a brand or workspace icon,
 * else the emoji of that shortcode. The emoji catalogue and the icon list are each fetched once,
 * and only when some room has an icon.
 */
export function resolveRoomIcon(iconName: string): Promise<RoomIconLook> {
  const known = resolved.get(iconName);

  if (known !== undefined) {
    return Promise.resolve(known);
  }

  return Promise.all([loadEmoji(), loadCustomIcons().catch(() => [])]).then(([emoji, custom]) => {
    const look = roomIconLook(iconName, emoji, custom);

    resolved.set(iconName, look);

    return look;
  });
}

const NONE: RoomIconLook = { kind: "none" };

export function useRoomIconLook(iconName: string | null): RoomIconLook {
  const [look, setLook] = useState<{ name: string | null; look: RoomIconLook }>(() => ({
    name: iconName,
    look: iconName === null ? NONE : (resolved.get(iconName) ?? NONE),
  }));

  if (look.name !== iconName) {
    setLook({ name: iconName, look: iconName === null ? NONE : (resolved.get(iconName) ?? NONE) });
  }

  useEffect(() => {
    if (iconName === null || resolved.has(iconName)) {
      return;
    }

    let live = true;

    void resolveRoomIcon(iconName).then((next) => {
      if (live) setLook({ name: iconName, look: next });
    });

    return () => {
      live = false;
    };
  }, [iconName]);

  return look.look;
}

interface RoomGlyphProps {
  readonly kind: RoomKind;
  readonly iconName: string | null;
  readonly size: number;
  readonly className?: string;
}

/**
 * A room's leading glyph: its own icon (an emoji or a workspace icon) when it has one, else its
 * kind's (`#`, the lock, the speaker…). Decorative: the name next to it says which room it is.
 */
export function RoomGlyph({ kind, iconName, size, className }: RoomGlyphProps) {
  const look = useRoomIconLook(iconName);
  const classes = `room-glyph${className === undefined ? "" : ` ${className}`}`;

  switch (look.kind) {
    case "emoji":
      return (
        <span
          className={`${classes} room-glyph-emoji enter-fade`}
          style={{ fontSize: Math.round(size * 0.92), width: size, height: size }}
          aria-hidden="true"
        >
          {look.char}
        </span>
      );
    case "image":
      return (
        <img
          className={`${classes} room-glyph-image enter-fade`}
          src={look.url}
          alt=""
          width={size}
          height={size}
          draggable={false}
        />
      );
    case "none":
      return (
        <Icon
          name={ROOM_KIND_ICON[kind]}
          size={size}
          {...(className === undefined ? {} : { className })}
        />
      );
  }
}
