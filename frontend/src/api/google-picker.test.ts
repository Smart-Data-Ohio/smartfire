import { afterEach, describe, expect, it, vi } from "vitest";
import { prepareGooglePicker } from "./google-picker.ts";

const config = {
  clientId: "client",
  apiKey: "developer-key",
  projectNumber: "123456",
  accountEmail: null,
};

const connected = { ...config, accountEmail: "connected@example.com" };

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

    return auth.callback(reply);
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
    expect(fake.oauth.initTokenClient).toHaveBeenCalledWith(
      expect.not.objectContaining({ login_hint: expect.any(String), hint: expect.any(String) }),
    );
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

  it("hints the connected account and verifies its token before opening the Picker", async () => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };

    const lookup = vi
      .spyOn(globalThis, "fetch")
      .mockResolvedValue(Response.json({ user: { emailAddress: "CONNECTED@example.com" } }));

    const session = await prepareGooglePicker(connected);
    const selection = session.choose();

    expect(fake.oauth.initTokenClient).toHaveBeenCalledWith(
      expect.objectContaining({
        login_hint: "connected@example.com",
        hint: "connected@example.com",
        scope,
        include_granted_scopes: false,
      }),
    );
    expect(fake.visible).not.toHaveBeenCalled();
    await fake.authorize({ access_token: "verified-token", scope });
    expect(lookup).toHaveBeenCalledWith(
      "https://www.googleapis.com/drive/v3/about?fields=user(emailAddress)",
      {
        headers: { Authorization: "Bearer verified-token" },
        signal: expect.any(AbortSignal),
      },
    );
    expect(fake.visible).toHaveBeenCalledWith(true);
    fake.picked({ action: "picked", docs: [{ id: "ConnectedFile123" }] });
    expect(await selection).toMatchObject({ id: "ConnectedFile123" });
    session.dispose();
  });

  it("discards a different account's token without opening the Picker, then allows a matching retry", async () => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
    vi.spyOn(globalThis, "fetch")
      .mockResolvedValueOnce(Response.json({ user: { emailAddress: "other@example.com" } }))
      .mockResolvedValueOnce(Response.json({ user: { emailAddress: connected.accountEmail } }));
    const session = await prepareGooglePicker(connected);

    const mismatch = expect(session.choose()).rejects.toThrow(
      "Pick files from connected@example.com, the Google account connected to Smartfire",
    );

    await fake.authorize({ access_token: "wrong-account-token", scope });
    await mismatch;
    expect(fake.token).not.toHaveBeenCalled();
    expect(fake.visible).not.toHaveBeenCalled();
    expect(document.documentElement.innerHTML).not.toContain("wrong-account-token");

    const retry = session.choose();
    await fake.authorize({ access_token: "matching-account-token", scope });
    fake.picked({ action: "picked", docs: [{ id: "ConnectedFile123" }] });
    expect(await retry).toMatchObject({ id: "ConnectedFile123" });
    expect(fake.token).toHaveBeenCalledExactlyOnceWith("matching-account-token");
    session.dispose();
  });

  it.each([
    Response.json({ error: "unauthorized" }, { status: 401 }),
    Response.json({ user: {} }),
    Response.json({ user: { emailAddress: " " } }),
    new Response("invalid JSON"),
  ])("keeps the Picker closed when the account cannot be verified (%#)", async (response) => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
    vi.spyOn(globalThis, "fetch").mockResolvedValue(response);
    const session = await prepareGooglePicker(connected);

    const failure = expect(session.choose()).rejects.toThrow(
      "The Google account could not be verified. Try again.",
    );

    await fake.authorize({ access_token: "unverified-token", scope });
    await failure;
    expect(fake.token).not.toHaveBeenCalled();
    expect(fake.visible).not.toHaveBeenCalled();
    session.dispose();
  });

  it("aborts an account lookup on disposal and ignores its late response", async () => {
    const fake = google();
    window.google = { accounts: { oauth2: fake.oauth }, picker: fake.api };
    let respond: (response: Response) => void = () => {};

    const lookup = vi.spyOn(globalThis, "fetch").mockImplementation(
      () =>
        new Promise((resolve) => {
          respond = resolve;
        }),
    );

    const session = await prepareGooglePicker(connected);
    const selection = session.choose();
    const authorization = fake.authorize({ access_token: "cancelled-token", scope });
    const signal = lookup.mock.calls[0]?.[1]?.signal;

    session.dispose();
    expect(await selection).toBeNull();
    expect(signal?.aborted).toBe(true);
    respond(Response.json({ user: { emailAddress: connected.accountEmail } }));
    await authorization;
    expect(fake.token).not.toHaveBeenCalled();
    expect(fake.visible).not.toHaveBeenCalled();
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
