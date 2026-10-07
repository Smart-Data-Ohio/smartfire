import { MenuItem, MenuSeparator } from "../../ui/menu.tsx";
import { schedulePresets } from "../composer/schedule/presets.ts";

interface ReminderItemsProps {
  /** Whether a reminder is set (and can be cleared). */
  readonly hasReminder: boolean;
  /** A preset: the composer's scheduled-send presets (in an hour, tomorrow, Monday morning). */
  readonly onRemind: (at: Date) => void;
  /** "Custom…": the date-and-time picker. */
  readonly onCustom: () => void;
  readonly onClear: () => void;
}

/**
 * "Remind me": Slack's presets for a saved message's reminder, then a custom time, then (when
 * one is set) clearing it. The same presets as the composer's scheduled send.
 */
export function ReminderItems({ hasReminder, onRemind, onCustom, onClear }: ReminderItemsProps) {
  const presets = schedulePresets(new Date());

  return (
    <>
      {presets.map((preset) => (
        <MenuItem key={preset.id} icon="alarm-clock" onSelect={() => onRemind(preset.at)}>
          {preset.label}
        </MenuItem>
      ))}
      <MenuItem icon="calendar-clock" onSelect={onCustom}>
        Custom…
      </MenuItem>
      {hasReminder ? (
        <>
          <MenuSeparator />
          <MenuItem icon="bell-off" onSelect={onClear}>
            Clear reminder
          </MenuItem>
        </>
      ) : null}
    </>
  );
}
