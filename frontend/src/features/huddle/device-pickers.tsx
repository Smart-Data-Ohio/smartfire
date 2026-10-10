import { useId, useState } from "react";
import type { StreamQuality } from "../../gen/StreamQuality.ts";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { audioOutputSupported, type DeviceLists, EMPTY_LABEL } from "./engine/devices.ts";
import { type DevicePreferences, loadShareQuality } from "./engine/preferences.ts";
import { knownStreamQuality, STREAM_QUALITIES } from "./engine/screen-quality.ts";
import type { DeviceKind } from "./engine/transport.ts";

const LABELS: Readonly<Record<DeviceKind, string>> = {
  audioinput: "Microphone",
  audiooutput: "Speaker",
  videoinput: "Camera",
};

const ICONS: Readonly<Record<DeviceKind, IconName>> = {
  audioinput: "mic",
  audiooutput: "volume",
  videoinput: "video",
};

interface DevicePickerProps {
  readonly kind: DeviceKind;
  readonly devices: DeviceLists;
  readonly selected: DevicePreferences;
  readonly onSelect: (kind: DeviceKind, deviceId: string) => void;
  /** An extra first option with an empty id ("No camera preview"). */
  readonly none?: string | undefined;
  readonly disabled?: boolean;
}

function DevicePicker({ kind, devices, selected, onSelect, none, disabled }: DevicePickerProps) {
  const id = useId();
  const options = devices[kind];
  const empty = options.length === 0 && none === undefined;

  return (
    <div className="huddle-picker">
      <label htmlFor={id} className="huddle-picker-label">
        <Icon name={ICONS[kind]} size={14} />
        {LABELS[kind]}
      </label>
      <select
        id={id}
        className="huddle-select"
        value={selected[kind]}
        disabled={disabled === true || empty}
        onChange={(event) => onSelect(kind, event.currentTarget.value)}
      >
        {none === undefined ? null : <option value="">{none}</option>}
        {empty ? <option value="">{EMPTY_LABEL[kind]}</option> : null}
        {options.map((device) => (
          <option key={device.deviceId} value={device.deviceId}>
            {device.label}
          </option>
        ))}
      </select>
    </div>
  );
}

interface DevicePickersProps {
  readonly devices: DeviceLists;
  readonly selected: DevicePreferences;
  readonly onSelect: (kind: DeviceKind, deviceId: string) => void;
  /** The device check offers "no camera preview"; a call switches only between real devices. */
  readonly cameraOptional: boolean;
}

/** The microphone, speaker (where the browser can switch it) and camera pickers. */
export function DevicePickers({ devices, selected, onSelect, cameraOptional }: DevicePickersProps) {
  return (
    <div className="huddle-pickers">
      <DevicePicker kind="audioinput" devices={devices} selected={selected} onSelect={onSelect} />
      {audioOutputSupported() ? (
        <DevicePicker
          kind="audiooutput"
          devices={devices}
          selected={selected}
          onSelect={onSelect}
        />
      ) : null}
      <DevicePicker
        kind="videoinput"
        devices={devices}
        selected={selected}
        onSelect={onSelect}
        none={cameraOptional ? "No camera preview" : undefined}
      />
    </div>
  );
}

interface ShareQualityPickerProps {
  readonly onSelect: (quality: StreamQuality) => void;
  /** While a share runs it keeps the quality it started with. */
  readonly sharing: boolean;
}

/** The quality an ordinary screen share starts at, laid out like the device pickers. */
export function ShareQualityPicker({ onSelect, sharing }: ShareQualityPickerProps) {
  const id = useId();
  const [quality, setQuality] = useState(loadShareQuality);

  return (
    <div className="huddle-picker">
      <label htmlFor={id} className="huddle-picker-label">
        <Icon name="screen-share" size={14} />
        Screen share quality
      </label>
      <select
        id={id}
        className="huddle-select"
        value={quality}
        disabled={sharing}
        title={sharing ? "Stop sharing to change the quality" : undefined}
        onChange={(event) => {
          const next = knownStreamQuality(event.currentTarget.value);

          if (next !== null) {
            setQuality(next);
            onSelect(next);
          }
        }}
      >
        {STREAM_QUALITIES.map((value) => (
          <option key={value} value={value}>
            {value}
          </option>
        ))}
      </select>
    </div>
  );
}

/** The microphone level as a bar, 0–100. */
export function MeterBar({ level, label }: { readonly level: number; readonly label: string }) {
  const clamped = Math.max(0, Math.min(100, level));

  return (
    // biome-ignore lint/a11y/useSemanticElements: a <meter> can't take the animated fill the dock and device check share
    <div
      className="huddle-meter"
      role="meter"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(clamped)}
    >
      <span className="huddle-meter-fill" style={{ "--meter": clamped / 100 }} />
    </div>
  );
}
