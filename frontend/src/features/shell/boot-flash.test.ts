import { afterEach, describe, expect, it } from "vitest";
import type { Boot } from "../../store/model.ts";
import { removeToast, toastSnapshot } from "../../ui/toast-store.ts";
import { showBootFlash } from "./boot-flash.ts";

const base: Boot = {
  user: { id: 1, name: "Ada", avatarUrl: "/a.png" },
  account: {
    name: "Signal",
    logoUrl: null,
    logoStillUrl: null,
    bannerUrl: null,
    bannerStillUrl: null,
    uploadLimitBytes: 100 * 1024 * 1024,
  },
  theme: "system",
  textSize: "default",
  customStyles: null,
  cableUrl: "/cable",
  serviceWorkerUrl: null,
  version: "2.0.0",
  revision: null,
  appearancePreferences: null,
};

describe("showBootFlash", () => {
  afterEach(() => {
    for (const { id } of toastSnapshot()) {
      removeToast(id);
    }
  });

  it("toasts a notice as success and an alert as danger, once per boot", () => {
    const notice: Boot = { ...base, flash: { kind: "notice", message: "GitHub is connected." } };
    const alert: Boot = { ...base, flash: { kind: "alert", message: "Google said no." } };

    showBootFlash(notice);
    showBootFlash(notice);
    showBootFlash(alert);

    expect(toastSnapshot().map(({ title, tone }) => ({ title, tone }))).toEqual([
      { title: "GitHub is connected.", tone: "success" },
      { title: "Google said no.", tone: "danger" },
    ]);
  });

  it("shows nothing without a flash", () => {
    showBootFlash(null);
    showBootFlash(base);
    showBootFlash({ ...base, flash: null });

    expect(toastSnapshot()).toEqual([]);
  });
});
