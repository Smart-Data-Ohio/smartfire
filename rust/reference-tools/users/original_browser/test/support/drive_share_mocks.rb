# Shared browser doubles for the enhanced Drive share flow: the Identity
# Services token client, the Picker, and Drive REST are all installed on
# window, so tests never contact live Google and never change real Drive
# permissions. Included by the share-flow tests and the attach-menu tests.
module DriveShareMocks
  FILE_ID = "1AbcDefGhIjKlMnOpQrSt"

  DRIVE_FILE_SCOPE = "https://www.googleapis.com/auth/drive.file"

  # Installs window.google (GIS + Picker) and wraps window.fetch so Drive
  # REST is answered from the scenario. Same-origin recipients calls pass
  # through to the app but are logged. Receives the scenario JSON as its
  # first script argument.
  MOCK_JS = <<~JS
    const scenario = JSON.parse(arguments[0]);
    const mock = window.__driveShareMock = {
      scenario,
      initTokenCalls: [],
      requestTokenCalls: [],
      pickerBuilds: [],
      pickerVisible: [],
      driveCalls: [],
      createOrder: [],
      recipientsCalls: [],
      listFailedOnce: false,
      createFailedOnce: false
    };

    window.google = window.google || {};
    window.google.accounts = { oauth2: {
      initTokenClient(config) {
        mock.lastTokenConfig = config;
        mock.initTokenCalls.push({
          scope: config.scope,
          include_granted_scopes: config.include_granted_scopes,
          client_id: config.client_id
        });
        return {
          requestAccessToken(override) {
            mock.requestTokenCalls.push(override || {});
            if (mock.scenario.tokenErrorCallback) {
              const err = JSON.parse(JSON.stringify(mock.scenario.tokenErrorCallback));
              setTimeout(() => config.error_callback && config.error_callback(err), 0);
              return;
            }
            const response = JSON.parse(JSON.stringify(mock.scenario.token));
            setTimeout(() => config.callback(response), 0);
          }
        };
      },
      hasGrantedAllScopes: (response) => !response.error && !!response.access_token
    }};

    class MockDocsView {
      constructor(viewId) { this.viewId = viewId; }
      setIncludeFolders(value) { return this; }
      setSelectFolderEnabled(value) { return this; }
      setMode(value) { return this; }
    }
    class MockBuilder {
      constructor() { this.args = {}; }
      addView(view) { return this; }
      setOAuthToken(token) { this.args.oauthToken = token; return this; }
      setDeveloperKey(key) { this.args.developerKey = key; return this; }
      setAppId(id) { this.args.appId = id; return this; }
      setCallback(callback) { this.args.callback = callback; return this; }
      setOrigin(origin) { this.args.origin = origin; return this; }
      setTitle(title) { return this; }
      build() {
        mock.pickerBuilds.push({ ...this.args, callback: !!this.args.callback });
        const callback = this.args.callback;
        mock.lastPickerCallback = callback;
        return { setVisible: (visible) => {
          mock.pickerVisible.push(visible);
          if (!visible) return;
          setTimeout(() => {
            const pick = mock.scenario.picker;
            if (pick === "manual") return;
            if (pick === "cancel") callback({ action: "cancel" });
            else callback({ action: "picked", docs: [pick] });
          }, 0);
        }};
      }
    }
    window.google.picker = {
      DocsView: MockDocsView,
      ViewId: { DOCS: "docs" },
      DocsViewMode: { LIST: "list" },
      PickerBuilder: MockBuilder,
      Response: { ACTION: "action", DOCUMENTS: "docs" },
      Action: { PICKED: "picked", CANCEL: "cancel" },
      Document: { ID: "id", NAME: "name", MIME_TYPE: "mimeType" }
    };

    window.__driveShareRespond = (url, options) => {
      const live = window.__driveShareMock;
      const expectedAuth = `Bearer ${live.scenario.token.access_token}`;
      if ((options.headers || {}).Authorization !== expectedAuth) {
        return { status: 401, body: { error: { code: 401 } } };
      }
      const path = new URL(url).pathname;
      const method = options.method || "GET";
      if (method === "POST" && path.endsWith("/permissions")) {
        const body = JSON.parse(options.body);
        live.createOrder.push(body.emailAddress);
        if (live.scenario.create401OnAttempt && live.createOrder.length === live.scenario.create401OnAttempt && !live.createFailedOnce) {
          live.createFailedOnce = true;
          return { status: 401, body: { error: { code: 401 } } };
        }
        if (live.scenario.fillStripOnCreate) {
          const strip = document.querySelector(".composer__drive-attachments");
          for (let i = 0; i < 10; i++) {
            const chip = document.createElement("span");
            chip.className = "drive-attachment-chip";
            const input = document.createElement("input");
            input.type = "hidden";
            input.name = "message[drive_file_ids][]";
            input.value = `midflight${i}1`;
            chip.append(input);
            strip.append(chip);
          }
          live.scenario.fillStripOnCreate = false;
        }
        const behavior = (live.scenario.creates || {})[body.emailAddress] || "ok";
        if (behavior === "ok") return { status: 200, body: { id: `perm-${body.emailAddress}`, role: "reader" } };
        return { status: behavior.status || 403, body: { error: { errors: [{ reason: behavior.reason || "forbidden" }], code: behavior.status || 403 } } };
      }
      if (method === "GET" && path.endsWith("/permissions")) {
        if (live.scenario.failFirstListWith401 && !live.listFailedOnce) {
          live.listFailedOnce = true;
          return { status: 401, body: { error: { code: 401 } } };
        }
        const token = new URL(url).searchParams.get("pageToken");
        const index = token ? parseInt(token.replace("page", ""), 10) : 0;
        const pages = live.scenario.permissionPages || [[]];
        const body = { permissions: pages[index] || [] };
        if (index + 1 < pages.length) body.nextPageToken = `page${index + 1}`;
        return { status: 200, body };
      }
      if (method === "GET" && path.startsWith("/drive/v3/files/")) {
        if (live.scenario.file && live.scenario.file.error) {
          return { status: live.scenario.file.error.status || 403, body: { error: { code: live.scenario.file.error.status || 403 } } };
        }
        return { status: 200, body: live.scenario.file };
      }
      return { status: 404, body: {} };
    };

    if (!window.__driveShareFetchWrapped) {
      window.__driveShareFetchWrapped = true;
      const realFetch = window.fetch.bind(window);
      window.fetch = (input, options = {}) => {
        const live = window.__driveShareMock;
        const url = String(input && input.url ? input.url : input);
        if (url.startsWith("https://www.googleapis.com/drive/v3/")) {
          live.driveCalls.push({
            url, method: options.method || "GET",
            auth: (options.headers || {}).Authorization || null,
            body: typeof options.body === "string" ? options.body : null
          });
          const { status, body } = window.__driveShareRespond(url, options);
          return Promise.resolve(new Response(JSON.stringify(body), { status }));
        }
        if (url.includes("/drive_recipients")) {
          live.recipientsCalls.push({ url, method: options.method || "GET", body: options.body || null });
        }
        return realFetch(input, options);
      };
    }
  JS

  def drive_scenario(overrides = {})
    {
      "token" => {
        "access_token" => "mock-token-1", "token_type" => "Bearer",
        "expires_in" => 3600, "scope" => DRIVE_FILE_SCOPE
      },
      "picker" => { "id" => FILE_ID, "name" => "Q3 Planning", "mimeType" => "application/vnd.google-apps.document" },
      "file" => {
        "id" => FILE_ID, "name" => "Q3 Planning",
        "mimeType" => "application/vnd.google-apps.document",
        "capabilities" => { "canShare" => true }
      },
      "permissionPages" => [ [] ],
      "creates" => {},
      "failFirstListWith401" => false
    }.merge(overrides)
  end

  def inject_drive_share_mocks(scenario)
    page.execute_script(MOCK_JS, scenario.to_json)
  end
end
