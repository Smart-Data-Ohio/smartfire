import { Schema } from "effect";
import type { DrivePick } from "../features/composer/drive-picker.ts";
import type { DrivePickerConfig } from "../gen/DrivePickerConfig.ts";

const DRIVE_SCOPE = "https://www.googleapis.com/auth/drive.file";

const CANCEL_CODES = new Set(["popup_closed", "popup_failed_to_open", "access_denied"]);

const TokenReply = Schema.Struct({
  access_token: Schema.optional(Schema.String),
  scope: Schema.optional(Schema.String),
  error: Schema.optional(Schema.String),
});

const PickerReply = Schema.Struct({
  action: Schema.String,
  docs: Schema.optional(
    Schema.Array(
      Schema.Struct({
        id: Schema.String,
        name: Schema.optional(Schema.String),
        mimeType: Schema.optional(Schema.String),
      }),
    ),
  ),
});

interface GoogleDocsView {
  setIncludeFolders(include: boolean): GoogleDocsView;
  setSelectFolderEnabled(enabled: boolean): GoogleDocsView;
  setMode(mode: string): GoogleDocsView;
}

interface GooglePickerWindow {
  setVisible(visible: boolean): void;
  dispose(): void;
}

interface GooglePickerBuilder {
  addView(view: GoogleDocsView): GooglePickerBuilder;
  setOAuthToken(token: string): GooglePickerBuilder;
  setDeveloperKey(key: string): GooglePickerBuilder;
  setAppId(id: string): GooglePickerBuilder;
  setOrigin(origin: string): GooglePickerBuilder;
  setTitle(title: string): GooglePickerBuilder;
  setCallback(callback: (reply: typeof PickerReply.Type) => void): GooglePickerBuilder;
  build(): GooglePickerWindow;
}

interface GoogleOAuth {
  initTokenClient(config: {
    client_id: string;
    scope: string;
    include_granted_scopes: boolean;
    callback: (reply: typeof TokenReply.Type) => void;
    error_callback: (failure: { type: string }) => void;
  }): { requestAccessToken(config: { prompt: string }): void };
  hasGrantedAllScopes(reply: typeof TokenReply.Type, scope: string): boolean;
}

declare global {
  interface Window {
    google?: {
      accounts?: { oauth2: GoogleOAuth };
      picker?: {
        DocsView: new (id: string) => GoogleDocsView;
        PickerBuilder: new () => GooglePickerBuilder;
        ViewId: { DOCS: string };
        DocsViewMode: { LIST: string };
      };
    };
    gapi?: {
      load(
        name: string,
        callbacks: {
          callback: () => void;
          onerror: () => void;
          timeout: number;
          ontimeout: () => void;
        },
      ): void;
    };
  }
}

export interface PickerSession {
  /** Called by a click so Google's authorization popup keeps browser activation. */
  choose(): Promise<DrivePick | null>;
  dispose(): void;
}

function kindOf(mime: string): string {
  switch (mime) {
    case "application/vnd.google-apps.document":
      return "document";
    case "application/vnd.google-apps.spreadsheet":
      return "spreadsheet";
    case "application/vnd.google-apps.presentation":
      return "presentation";
    case "application/vnd.google-apps.form":
      return "form";
    case "application/vnd.google-apps.folder":
      return "folder";
    case "application/vnd.google-apps.shortcut":
      return "shortcut";
    case "application/pdf":
      return "pdf";
    default:
      return "file";
  }
}

function loadScript(url: string): Promise<void> {
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    const timer = window.setTimeout(() => fail(), 15_000);

    const fail = () => {
      window.clearTimeout(timer);
      script.remove();
      reject(new Error("Google Drive could not be reached. Check your connection."));
    };

    script.src = url;
    script.async = true;
    script.onload = () => {
      window.clearTimeout(timer);
      resolve();
    };

    script.onerror = fail;
    document.head.append(script);
  });
}

let scripts: Promise<void> | null = null;

function loadGoogle(): Promise<void> {
  scripts ??= (async () => {
    if (!window.google?.accounts) await loadScript("https://accounts.google.com/gsi/client");

    if (!window.google?.picker) {
      if (!window.gapi) await loadScript("https://apis.google.com/js/api.js");
      const gapi = window.gapi;

      if (!gapi) throw new Error("Google Drive did not load.");
      await new Promise<void>((resolve, reject) => {
        const fail = () => reject(new Error("Google Drive did not load."));
        gapi.load("picker", { callback: resolve, onerror: fail, timeout: 15_000, ontimeout: fail });
      });
    }
  })().catch((error: Error) => {
    scripts = null;
    throw error;
  });

  return scripts;
}

export async function prepareGooglePicker(config: DrivePickerConfig): Promise<PickerSession> {
  if (import.meta.env.MODE === "mock") {
    const mock = await import("../../mock/google-picker.ts");

    return mock.preparePicker();
  }

  await loadGoogle();
  const oauth = window.google?.accounts?.oauth2;
  const api = window.google?.picker;

  if (!oauth || !api) throw new Error("Google Drive did not load.");
  let picker: GooglePickerWindow | null = null;
  let cancel = () => {};

  let disposed = false;

  const hide = () => {
    picker?.setVisible(false);
    picker?.dispose();
    picker = null;
  };

  return {
    choose: () =>
      new Promise((resolve, reject) => {
        if (disposed) {
          resolve(null);

          return;
        }

        const finish = (file: DrivePick | null) => {
          hide();
          cancel = () => {};

          resolve(file);
        };

        const fail = (message: string) => {
          hide();
          cancel = () => {};

          reject(new Error(message));
        };

        cancel = () => finish(null);

        try {
          const client = oauth.initTokenClient({
            client_id: config.clientId,
            scope: DRIVE_SCOPE,
            include_granted_scopes: false,
            error_callback: (error) => {
              if (disposed) return;

              if (CANCEL_CODES.has(error.type)) finish(null);
              else fail("Google authorization failed. Try again.");
            },
            callback: (response) => {
              if (disposed) return;

              try {
                const token = Schema.decodeUnknownSync(TokenReply)(response);

                if (token.error && CANCEL_CODES.has(token.error)) {
                  finish(null);

                  return;
                }

                if (
                  token.error ||
                  !token.access_token ||
                  !oauth.hasGrantedAllScopes(token, DRIVE_SCOPE)
                ) {
                  fail("Drive access was not granted. Try again.");

                  return;
                }

                const view = new api.DocsView(api.ViewId.DOCS)
                  .setIncludeFolders(true)
                  .setSelectFolderEnabled(false)
                  .setMode(api.DocsViewMode.LIST);

                // The token stays in this callback and Google's Picker, never the DOM or storage.
                picker = new api.PickerBuilder()
                  .addView(view)
                  .setOAuthToken(token.access_token)
                  .setDeveloperKey(config.apiKey)
                  .setAppId(config.projectNumber)
                  .setOrigin(window.location.origin)
                  .setTitle("Choose a Drive file")
                  .setCallback((reply) => {
                    if (disposed) return;

                    try {
                      const data = Schema.decodeUnknownSync(PickerReply)(reply);

                      if (data.action === "cancel") {
                        finish(null);

                        return;
                      }

                      if (data.action !== "picked") return;
                      const doc = data.docs?.[0];

                      if (!doc || !/^[A-Za-z0-9_-]{10,}$/.test(doc.id)) {
                        fail("That selection could not be attached. Choose again.");

                        return;
                      }

                      finish({
                        id: doc.id,
                        name: doc.name || "Google Drive file",
                        kind: kindOf(doc.mimeType ?? ""),
                        url: `https://drive.google.com/open?id=${doc.id}`,
                      });
                    } catch {
                      fail("That selection could not be attached. Choose again.");
                    }
                  })
                  .build();
                picker.setVisible(true);
              } catch {
                fail("The Drive window could not be opened. Try again.");
              }
            },
          });

          client.requestAccessToken({ prompt: "" });
        } catch {
          fail("Google authorization failed to start. Try again.");
        }
      }),
    dispose: () => {
      disposed = true;
      cancel();
      hide();
    },
  };
}
