import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { CameraQualityPicker, DevicePickers } from "./device-pickers.tsx";
import type { CameraCapabilities, CameraQuality } from "./engine/camera-quality.ts";
import {
  CAMERA_QUALITY_STORAGE_KEY,
  loadCameraQuality,
  storeCameraQuality,
} from "./engine/preferences.ts";

const FULL_HD_CAMERA: CameraCapabilities = {
  width: { min: 160, max: 1920 },
  height: { min: 90, max: 1080 },
  frameRate: { min: 1, max: 60 },
};

beforeEach(() => localStorage.clear());

describe("camera quality picker", () => {
  it("shows the applied fallback when capture could not deliver the requested combined mode", () => {
    storeCameraQuality("1080p60");

    const view = render(
      <CameraQualityPicker
        quality="1080p60"
        capabilities={FULL_HD_CAMERA}
        busy={true}
        onSelect={vi.fn()}
      />,
    );

    view.rerender(
      <CameraQualityPicker
        quality="1080p30"
        capabilities={FULL_HD_CAMERA}
        busy={false}
        onSelect={vi.fn()}
      />,
    );
    expect(screen.getByRole("combobox", { name: "Camera quality" })).toHaveProperty(
      "value",
      "1080p30",
    );
  });

  it("offers only the default while camera capabilities are unknown", () => {
    const onSelect = vi.fn();
    render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={null}
        busy={false}
        onSelect={onSelect}
      />,
    );

    const picker = screen.getByRole("combobox", { name: "Camera quality" });

    expect(picker.getAttribute("class")).toBe("huddle-select");
    expect(within(picker).getAllByRole("option")).toHaveLength(1);
    expect(within(picker).getByRole("option", { name: "720p30" }).hasAttribute("disabled")).toBe(
      false,
    );
    expect(within(picker).queryByRole("option", { name: "1080p30" })).toBeNull();
    expect(within(picker).queryByRole("option", { name: "1080p60" })).toBeNull();
    expect(onSelect).not.toHaveBeenCalled();
    expect(localStorage.getItem(CAMERA_QUALITY_STORAGE_KEY)).toBeNull();
  });

  it("hides 60 fps when the active camera only reports 30 fps", () => {
    render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={{ ...FULL_HD_CAMERA, frameRate: { min: 1, max: 30 } }}
        busy={false}
        onSelect={vi.fn()}
      />,
    );

    expect(screen.getByRole("option", { name: "1080p30" }).hasAttribute("disabled")).toBe(false);
    expect(screen.queryByRole("option", { name: "1080p60" })).toBeNull();
  });

  it("preserves an unavailable preferred mode until the camera reports its bounds", () => {
    storeCameraQuality("1080p60");
    const onSelect = vi.fn();

    const view = render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={null}
        busy={false}
        onSelect={onSelect}
      />,
    );

    const saved = screen.getByRole("option", { name: "1080p60 (unavailable)" });

    expect(saved.hasAttribute("disabled")).toBe(true);
    expect(saved instanceof HTMLOptionElement && saved.selected).toBe(true);
    expect(loadCameraQuality()).toBe("1080p60");
    expect(onSelect).not.toHaveBeenCalled();

    view.rerender(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={FULL_HD_CAMERA}
        busy={false}
        onSelect={onSelect}
      />,
    );
    const supported = screen.getByRole("option", { name: "1080p60" });

    expect(supported.hasAttribute("disabled")).toBe(false);
    expect(supported instanceof HTMLOptionElement && supported.selected).toBe(true);
    expect(onSelect).not.toHaveBeenCalled();
    expect(loadCameraQuality()).toBe("1080p60");
  });

  it("passes a selected supported mode to the controller and remembers its result on remount", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn((quality: CameraQuality) => storeCameraQuality(quality));

    const view = render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={FULL_HD_CAMERA}
        busy={false}
        onSelect={onSelect}
      />,
    );

    await user.selectOptions(screen.getByRole("combobox", { name: "Camera quality" }), "1080p30");

    expect(onSelect).toHaveBeenCalledExactlyOnceWith("1080p30");
    expect(loadCameraQuality()).toBe("1080p30");
    view.unmount();
    render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={FULL_HD_CAMERA}
        busy={false}
        onSelect={onSelect}
      />,
    );

    const saved = screen.getByRole("option", { name: "1080p30" });

    expect(saved instanceof HTMLOptionElement && saved.selected).toBe(true);
  });

  it("waits while a camera change is in progress", async () => {
    const user = userEvent.setup();
    const onSelect = vi.fn();
    render(
      <CameraQualityPicker
        quality={loadCameraQuality()}
        capabilities={FULL_HD_CAMERA}
        busy={true}
        onSelect={onSelect}
      />,
    );
    const picker = screen.getByRole("combobox", { name: "Camera quality" });

    expect(picker.hasAttribute("disabled")).toBe(true);
    await user.selectOptions(picker, "1080p60");
    expect(onSelect).not.toHaveBeenCalled();
    expect(loadCameraQuality()).toBe("720p30");
  });
});

it("places the optional camera quality picker directly after the camera device", () => {
  const props = {
    devices: {
      audioinput: [{ deviceId: "mic", label: "Desk microphone" }],
      audiooutput: [],
      videoinput: [{ deviceId: "camera", label: "Desk camera" }],
    },
    selected: { audioinput: "mic", audiooutput: "", videoinput: "camera" },
    cameraOptional: false,
    onSelect: vi.fn(),
  };

  const view = render(
    <DevicePickers
      {...props}
      cameraQuality={{
        quality: loadCameraQuality(),
        capabilities: FULL_HD_CAMERA,
        busy: false,
        onSelect: vi.fn(),
      }}
    />,
  );

  expect(screen.getByRole("combobox", { name: "Camera" }).parentElement?.nextElementSibling).toBe(
    screen.getByRole("combobox", { name: "Camera quality" }).parentElement,
  );
  view.rerender(<DevicePickers {...props} />);
  expect(screen.queryByRole("combobox", { name: "Camera quality" })).toBeNull();
  expect(screen.getByRole("combobox", { name: "Camera" })).toBeTruthy();
});
