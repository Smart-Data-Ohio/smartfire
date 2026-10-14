import { type ReactNode, useEffect, useState } from "react";
import type { BotAvatarSpec } from "../../gen/BotAvatarSpec.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import {
  BOT_AVATAR_ACCESSORIES,
  BOT_AVATAR_FACES,
  BOT_AVATAR_SWATCHES,
  BOT_AVATAR_TYPES,
  type BotAvatarAccessory,
  colorOf,
  describeSpec,
  ownColor,
  randomSpec,
  toneOf,
  typeLabel,
  withType,
} from "../../ui/bot-avatar-spec.ts";
import { Button } from "../../ui/button.tsx";
import type { AvatarChoice } from "./avatar-choice.ts";
import "./bot-avatar-picker.css";

interface BotAvatarPickerProps {
  /** Prefixes the inputs' ids and radio group names, unique on the page. */
  readonly id: string;
  /** The bot's name, for the preview's label. */
  readonly name: string;
  readonly value: AvatarChoice;
  readonly onChange: (choice: AvatarChoice) => void;
  /** An edited bot's own icon or picture, drawn, offered as "Keep current"; `null` on create. */
  readonly current?: ReactNode;
  readonly error?: string | undefined;
}

const SOURCES = [
  { kind: "bot", label: "Bot avatar" },
  { kind: "upload", label: "Upload an image" },
  { kind: "keep", label: "Keep current" },
] as const;

/**
 * The bot's picture: a bot avatar picked from the library's eighteen bodies (live previews, a
 * colour, a face and one thing to wear), an uploaded image, or what an edited bot already has.
 * Every choice is a native radio group, so arrow keys move through it and screen readers name it.
 */
export function BotAvatarPicker({
  id,
  name,
  value,
  onChange,
  current = null,
  error,
}: BotAvatarPickerProps) {
  // The last picked avatar and image survive a trip to the other choice and back.
  const [spec, setSpec] = useState<BotAvatarSpec>(() =>
    value.kind === "bot" ? value.spec : randomSpec(),
  );

  const [file, setFile] = useState<File | null>(value.kind === "upload" ? value.file : null);
  const shown = value.kind === "bot" ? value.spec : spec;
  const sources = SOURCES.filter((source) => source.kind !== "keep" || current !== null);

  const pick = (next: BotAvatarSpec) => {
    setSpec(next);
    onChange({ kind: "bot", spec: next });
  };

  const choose = (kind: AvatarChoice["kind"]) => {
    if (kind === "bot") onChange({ kind, spec: shown });
    else if (kind === "upload") onChange({ kind, file });
    else onChange({ kind });
  };

  return (
    <div className="bot-avatar-picker" data-source={value.kind}>
      <fieldset className="bot-avatar-sources">
        <legend className="settings-label">Picture</legend>
        <div className="bot-avatar-segmented">
          {sources.map((source) => (
            <label key={source.kind} className="bot-avatar-segment">
              <input
                type="radio"
                name={`${id}-source`}
                value={source.kind}
                checked={value.kind === source.kind}
                onChange={() => choose(source.kind)}
              />
              <span className="bot-avatar-segment-label">{source.label}</span>
            </label>
          ))}
        </div>
      </fieldset>
      {value.kind === "bot" ? (
        <BotEditor id={id} name={name} spec={shown} onPick={pick} />
      ) : value.kind === "upload" ? (
        <ImageUpload
          id={id}
          name={name}
          file={file}
          onFile={(next) => {
            setFile(next);
            onChange({ kind: "upload", file: next });
          }}
        />
      ) : current === null ? null : (
        <div className="bot-avatar-current">
          {current}
          <p className="settings-hint">Keeps the icon or picture it has now.</p>
        </div>
      )}
      {error === undefined ? null : <p className="settings-error">{error}</p>}
    </div>
  );
}

interface BotEditorProps {
  readonly id: string;
  readonly name: string;
  readonly spec: BotAvatarSpec;
  readonly onPick: (spec: BotAvatarSpec) => void;
}

function BotEditor({ id, name, spec, onPick }: BotEditorProps) {
  const color = colorOf(spec);

  const swatches = BOT_AVATAR_SWATCHES.some((swatch) => swatch.color === color)
    ? BOT_AVATAR_SWATCHES
    : [
        {
          color,
          label: color === ownColor(spec.type) ? `${typeLabel(spec.type)}'s own` : "Current",
        },
        ...BOT_AVATAR_SWATCHES,
      ];

  return (
    <div className="bot-avatar-editor" data-tone={toneOf(color)}>
      <div className="bot-avatar-preview">
        <AgentAvatar seed={id} name={name || "New bot"} size={88} spec={spec} live />
        <div className="bot-avatar-preview-text">
          <p className="bot-avatar-summary" aria-live="polite">
            {describeSpec(spec)}
          </p>
          <Button
            type="button"
            variant="secondary"
            size="sm"
            icon="refresh-cw"
            onClick={() => onPick({ ...randomSpec(), accessory: randomAccessory() })}
          >
            Shuffle
          </Button>
        </div>
      </div>
      <fieldset className="bot-avatar-group">
        <legend className="bot-avatar-legend">Body</legend>
        <div className="bot-avatar-bodies">
          {BOT_AVATAR_TYPES.map((entry) => {
            const option = withType(spec, entry.type);
            const selected = entry.type === spec.type;

            // A tile shows the shape: the picked colour and face, but nothing worn, which would
            // hide the silhouette the tile is there to choose (the preview shows it all).
            return (
              <label key={entry.type} className="bot-avatar-body">
                <input
                  type="radio"
                  name={`${id}-type`}
                  value={entry.type}
                  checked={selected}
                  onChange={() => onPick(option)}
                />
                <AgentAvatar
                  seed={`${id}-${entry.type}`}
                  name={entry.label}
                  size={BODY_TILE}
                  spec={{ ...option, accessory: null }}
                  live={selected}
                  decorative
                />
                <span className="bot-avatar-body-label">{entry.label}</span>
              </label>
            );
          })}
        </div>
      </fieldset>
      <fieldset className="bot-avatar-group">
        <legend className="bot-avatar-legend">Colour</legend>
        <div className="bot-avatar-swatches">
          {swatches.map((swatch) => (
            <label key={swatch.color} className="bot-avatar-swatch" title={swatch.label}>
              <input
                type="radio"
                name={`${id}-color`}
                value={swatch.color}
                aria-label={swatch.label}
                checked={swatch.color === color}
                onChange={() => onPick({ ...spec, color: swatch.color })}
              />
              <span
                className="bot-avatar-swatch-dot"
                style={{ "--swatch": swatch.color }}
                aria-hidden="true"
              />
            </label>
          ))}
        </div>
      </fieldset>
      <fieldset className="bot-avatar-group">
        <legend className="bot-avatar-legend">Face</legend>
        <div className="bot-avatar-segmented">
          {BOT_AVATAR_FACES.map((entry) => (
            <label key={entry.face} className="bot-avatar-segment">
              <input
                type="radio"
                name={`${id}-face`}
                value={entry.face}
                checked={spec.face === entry.face}
                onChange={() => onPick({ ...spec, face: entry.face })}
              />
              <span className="bot-avatar-segment-label">{entry.label}</span>
            </label>
          ))}
        </div>
      </fieldset>
      <fieldset className="bot-avatar-group">
        <legend className="bot-avatar-legend">Wearing</legend>
        <div className="bot-avatar-chips">
          {[{ accessory: null, label: "Nothing" }, ...BOT_AVATAR_ACCESSORIES].map((entry) => (
            <label key={entry.accessory ?? "none"} className="bot-avatar-chip">
              <input
                type="radio"
                name={`${id}-accessory`}
                value={entry.accessory ?? ""}
                checked={spec.accessory === entry.accessory}
                onChange={() => onPick({ ...spec, accessory: entry.accessory })}
              />
              <span className="bot-avatar-chip-label">{entry.label}</span>
            </label>
          ))}
        </div>
      </fieldset>
    </div>
  );
}

/** A body tile's drawing, in CSS pixels: five across on a 360 px phone. */
const BODY_TILE = 56;

/** One thing to wear about a third of the time, else nothing. */
function randomAccessory(): BotAvatarAccessory | null {
  const index = Math.floor(Math.random() * BOT_AVATAR_ACCESSORIES.length * 3);

  return BOT_AVATAR_ACCESSORIES[index]?.accessory ?? null;
}

interface ImageUploadProps {
  readonly id: string;
  readonly name: string;
  readonly file: File | null;
  readonly onFile: (file: File | null) => void;
}

function ImageUpload({ id, name, file, onFile }: ImageUploadProps) {
  const [preview, setPreview] = useState<string | null>(null);

  useEffect(() => {
    if (file === null) {
      setPreview(null);

      return;
    }

    const url = URL.createObjectURL(file);

    setPreview(url);

    return () => URL.revokeObjectURL(url);
  }, [file]);

  return (
    <div className="bot-avatar-upload">
      {preview === null ? null : (
        <AgentAvatar seed={id} name={name || "New bot"} size={56} picture={preview} />
      )}
      <div className="settings-field">
        <label className="visually-hidden" htmlFor={`${id}-file`}>
          Image
        </label>
        <input
          id={`${id}-file`}
          className="input"
          type="file"
          accept="image/*"
          onChange={(event) => onFile(event.target.files?.[0] ?? null)}
        />
        <p className="settings-hint">A square image works best.</p>
      </div>
    </div>
  );
}
