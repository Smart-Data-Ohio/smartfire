import { useId } from "react";
import { Icon } from "../../ui/icons/icon.tsx";
import { CHANNEL_OPTIONS, type Channel } from "./room-forms.ts";

interface KindCardsProps {
  readonly value: Channel;
  readonly onValueChange: (channel: Channel) => void;
  /** The channels the viewer may create; the others aren't offered. */
  readonly available: readonly Channel[];
}

/**
 * The channel type as a column of radio cards (Discord's create-channel list): glyph, name and a
 * line on what it's for, the picked one ringed in the accent. Native radios, so ↑/↓ move the
 * choice and the group is one Tab stop.
 */
export function KindCards({ value, onValueChange, available }: KindCardsProps) {
  const name = useId();

  return (
    <fieldset className="kind-cards">
      <legend className="room-form-legend">Channel type</legend>
      {CHANNEL_OPTIONS.filter((option) => available.includes(option.channel)).map((option) => (
        <label key={option.channel} className="kind-card" data-checked={option.channel === value}>
          <span className="kind-card-glyph" aria-hidden="true">
            <Icon name={option.icon} size={20} />
          </span>
          <span className="kind-card-text">
            <span className="kind-card-title">{option.title}</span>
            <span className="kind-card-blurb">{option.blurb}</span>
          </span>
          <input
            type="radio"
            className="kind-card-radio"
            name={name}
            value={option.channel}
            checked={option.channel === value}
            onChange={() => onValueChange(option.channel)}
          />
        </label>
      ))}
    </fieldset>
  );
}
