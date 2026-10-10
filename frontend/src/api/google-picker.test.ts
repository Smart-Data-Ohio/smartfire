import { afterEach, describe, expect, it, vi } from "vitest";
import { prepareGooglePicker } from "./google-picker.ts";

const config = { clientId: "client", apiKey: "developer-key", projectNumber: "123456" };

const scope = "https://www.googleapis.com/auth/drive.file";

function google() {
  type OAuth = NonNullable<NonNullable<Window["google"]>["accounts"]>["oauth2"];

  type TokenConfig = Parameters<OAuth["initTokenClient"]>[0];

  type PickerApi = NonNullable<NonNullable<Window["google"]>["picker"]>;

  type Callback = Parameters<InstanceType<PickerApi["PickerBuilder"]>["setCallback"]>[0];

  let auth: TokenConfig | null = null;
  let callback: Callback = () => {};

  const requestAccessToken = vi.fn();
  const visible = vi.fn();
  const dispose = vi.fn();
  const token = vi.fn();
  const key = vi.fn();
  const appId = vi.fn();
  const mode = vi.fn();
  const selectFolders = vi.fn();

  const oauth: OAuth = {
    initTokenClient: vi.fn((value) => {
      auth = value;

      return { requestAccessToken };
    }),
    hasGrantedAllScopes: vi.fn(() => true),
  };

  const api: PickerApi = {
    ViewId: { DOCS: "all-docs" },
    DocsViewMode: { LIST: "list" },
    DocsView: class {
      setIncludeFolders() {
        return this;
      }
      setSelectFolderEnabled(value: boolean) {
        selectFolders(value);

        return this;
      }
      setMode(value: string) {
        mode(value);

        return this;
      }
    },
    PickerBuilder: class {
      addView() {
        return this;
      }
      setOAuthToken(value: string) {
        token(value);

        return this;
      }
      setDeveloperKey(value: string) {
        key(value);

        return this;
      }
      setAppId(value: string) {
        appId(value);

        return this;
      }
      setOrigin() {
        return this;
      }
      setTitle() {
        return this;
      }
      setCallback(value: Callback) {
        callback = value;

        return this;
      }
      build() {
        return { setVisible: visible, dispose };
      }
    },
  };

  const authorize = (reply: Parameters<TokenConfig["callback"]>[0]) => {
    if (auth === null) throw new Error("No authorization requested");
    auth.callback(reply);
  };

  const picked = (reply: Parameters<Callback>[0]) => callback(reply);

  return {
    oauth,
    api,
    authorize,
    picked,
    requestAccessToken,
    visible,
    dispose,
    token,
    key,
    appId,
    mode,
    selectFolders,
  };
}

afterEach(() => {
  delete window.google;
  delete window.gapi;
  vi.restoreAllMocks();

  for (const script of document.querySelectorAll('script[src*="google.com"]')) script.remove();
});

describe("official Google Picker", () => {
  it("loads scripts on use, gets a drive.file token on a click, and attaches a validated canonical file", async () => {
    const fake = google();
    expect(document.querySelector('script[src*="google.com"]')).toBeNull();
    const pending = prepareGooglePicker(config);

    const gis = document.querySelector<HTMLScriptElement>(
      'script[src="https://accounts.google.com/gsi/client"]',
    );

    expect(gis).not.toBeNull();
    window.google = { accounts: { oauth2: fake.oauth } };
    gis?.dispatchEvent(new Event("load"));
    await vi.waitFor(() =>
      expect(
        document.querySelector('script[src="https://apis.google.com/js/api.js"]'),
      ).not.toBeNull(),
    );
    window.gapi = {
      load: (name, handlers) => {
        expect(name).toBe("picker");
        window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
        handlers.callback();
      },
    };
    document
      .querySelector('script[src="https://apis.google.com/js/api.js"]')
      ?.dispatchEvent(new Event("load"));
    const session = await pending;
    expect(fake.requestAccessToken).not.toHaveBeenCalled();
    const selection = session.choose();
    expect(fake.oauth.initTokenClient).toHaveBeenCalledWith(
      expect.objectContaining({ client_id: config.clientId, scope, include_granted_scopes: false }),
    );
    expect(fake.requestAccessToken).toHaveBeenCalledWith({ prompt: "" });
    fake.authorize({ access_token: "ephemeral-token", scope });
    expect(fake.token).toHaveBeenCalledWith("ephemeral-token");
    expect(fake.key).toHaveBeenCalledWith(config.apiKey);
    expect(fake.appId).toHaveBeenCalledWith(config.projectNumber);
    expect(fake.mode).toHaveBeenCalledWith("list");
    expect(fake.selectFolders).toHaveBeenCalledWith(false);
    fake.picked({
      action: "picked",
      docs: [
        {
          id: "ExistingFile123",
          name: "Existing file",
          mimeType: "application/vnd.google-apps.document",
        },
      ],
    });
    expect(await selection).toEqual({
      id: "ExistingFile123",
      name: "Existing file",
      kind: "document",
      url: "https://drive.google.com/open?id=ExistingFile123",
    });
    expect(fake.visible).toHaveBeenLastCalledWith(false);
    expect(fake.dispose).toHaveBeenCalledOnce();
    expect(document.documentElement.innerHTML).not.toContain("ephemeral-token");
    session.dispose();
  });

  it("rejects invalid ids and keeps shortcuts attach-only", async () => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
    const session = await prepareGooglePicker(config);
    const invalid = session.choose();
    fake.authorize({ access_token: "token", scope });
    fake.picked({ action: "picked", docs: [{ id: "bad/link" }] });
    await expect(invalid).rejects.toThrow("That selection could not be attached");
    const valid = session.choose();
    fake.authorize({ access_token: "token", scope });
    fake.picked({
      action: "picked",
      docs: [{ id: "ShortcutFile123", mimeType: "application/vnd.google-apps.shortcut" }],
    });
    expect(await valid).toMatchObject({ kind: "shortcut" });
    session.dispose();
  });

  it("cancels consent quietly and ignores authorization after disposal", async () => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
    const session = await prepareGooglePicker(config);
    const denied = session.choose();
    fake.authorize({ error: "access_denied" });
    expect(await denied).toBeNull();
    const pending = session.choose();
    session.dispose();
    fake.authorize({ access_token: "too-late", scope });
    expect(await pending).toBeNull();
    expect(fake.token).not.toHaveBeenCalled();
  });
});
