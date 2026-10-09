import { useState } from "react";
import { COARSE_QUERY, PHONE_QUERY } from "../../../lib/breakpoints.ts";
import { useLongPress } from "../../../lib/long-press.ts";
import { shortcutKeys } from "../../../lib/shortcuts.ts";
import { Button } from "../../../ui/button.tsx";
import type { IconName } from "../../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../../ui/menu.tsx";
import { Tooltip } from "../../../ui/tooltip.tsx";
import { type PresetId, type SchedulePreset, schedulePresets, timeLabel } from "./presets.ts";

const PRESET_ICONS: Readonly<Record<PresetId, IconName>> = {
  hour: "clock",
  tomorrow: "sun",
  monday: "calendar-clock",
};

interface SendButtonProps {
  readonly canSend: boolean;
  /** Waiting for uploads to finish before it sends: the icon swaps to a spinner. */
  readonly waiting: boolean;
  readonly onSend: () => void;
  /** `null` when scheduling isn't offered (a new thread's first reply). */
  readonly schedule: {
    readonly enabled: boolean;
    /** Why it's off, for the menu (attachments can't be scheduled). */
    readonly reason: string | null;
    readonly onPreset: (preset: SchedulePreset) => void;
    readonly onCustom: () => void;
  } | null;
}

/** A touch phone, where the chevron folds into send and a long press opens the schedule menu. */
function touchPhone(): boolean {
  return window.matchMedia(`${PHONE_QUERY} and ${COARSE_QUERY}`).matches;
}

/**
 * The split send button: send on the left (it swaps to a spinner while it waits for uploads, the
 * text-states-swap recipe inside Button), and a chevron with the schedule presets on the right.
 * On a touch phone the chevron folds away (schedule.css) and a long press on send opens the
 * presets; the chevron stays in the tab order and shows itself when the keyboard reaches it.
 */
export function SendButton({ canSend, waiting, onSend, schedule }: SendButtonProps) {
  const [menuOpen, setMenuOpen] = useState(false);
  const schedulable = schedule !== null && canSend && !waiting;

  const longPress = useLongPress(
    () => setMenuOpen(true),
    () => schedulable && touchPhone(),
  );

  const send = (
    <Tooltip content="Send" shortcut={shortcutKeys("send")} describe={false}>
      <Button
        variant={canSend || waiting ? "primary" : "ghost"}
        size="sm"
        icon="send"
        className="composer-send"
        aria-label={waiting ? "Sending when uploads finish" : "Send message"}
        aria-keyshortcuts="Enter"
        disabled={!canSend && !waiting}
        loading={waiting}
        onMouseDown={(event) => event.preventDefault()}
        {...longPress.handlers}
        onClick={() => {
          if (!longPress.endsLongPress()) {
            onSend();
          }
        }}
      />
    </Tooltip>
  );

  if (schedule === null) {
    return <div className="composer-send-group">{send}</div>;
  }

  const presets = schedulePresets(new Date());

  return (
    <div className="composer-send-group" data-split={canSend || waiting || undefined}>
      {send}
      <Menu
        open={menuOpen}
        onOpenChange={setMenuOpen}
        placement="top-end"
        label="Schedule message"
        trigger={(props) => (
          <Button
            {...props}
            variant={canSend || waiting ? "primary" : "ghost"}
            size="sm"
            icon="chevron-down"
            className="composer-send-more"
            aria-label="Schedule message"
            disabled={!canSend || waiting}
            onMouseDown={(event) => event.preventDefault()}
          />
        )}
      >
        <div className="schedule-menu-head" aria-hidden="true">
          {schedule.reason ?? "Schedule message"}
        </div>
        {presets.map((preset) => (
          <MenuItem
            key={preset.id}
            icon={PRESET_ICONS[preset.id]}
            detail={preset.id === "hour" ? timeLabel(preset.at) : undefined}
            disabled={!schedule.enabled}
            onSelect={() => schedule.onPreset(preset)}
          >
            {preset.label}
          </MenuItem>
        ))}
        <MenuSeparator />
        <MenuItem icon="pencil" disabled={!schedule.enabled} onSelect={schedule.onCustom}>
          Custom time…
        </MenuItem>
      </Menu>
    </div>
  );
}
