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
  seedPicker() {
    class MockDocsView {
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
        return { setVisible: (visible) => {
          mock.pickerVisible.push(visible);
          if (!visible) return;
          setTimeout(() => callback({ action: "picked", docs: [mock.scenario.picker] }), 0);
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
  }
};

window.google = window.google || {};
window.google.accounts = { oauth2: {
  initTokenClient(config) {
    mock.initTokenCalls.push({ scope: config.scope, include_granted_scopes: config.include_granted_scopes });
    return { requestAccessToken: (override) => {
      mock.requestTokenCalls.push(override || {});
      const response = JSON.parse(JSON.stringify(mock.scenario.token));
      setTimeout(() => config.callback(response), 0);
    }};
  },
  hasGrantedAllScopes: (response) => !response.error && !!response.access_token
}};
window.gapi = { load: (_modules, opts) => setTimeout(() => {
  if (mock.scenario.failPickerLoad) opts.onerror(new Error("load failed"));
  else { mock.seedPicker(); opts.callback(); }
}, 0) };

window.__driveShareRespond = (_url, _options) => ({ status: 200, body: window.__driveShareMock.scenario.file });

if (!window.__driveShareFetchWrapped) {
  window.__driveShareFetchWrapped = true;
  const realFetch = window.fetch.bind(window);
  window.fetch = (input, options = {}) => {
    const live = window.__driveShareMock;
    const url = String(input && input.url ? input.url : input);
    if (url.startsWith("https://www.googleapis.com/drive/v3/")) {
      live.driveCalls.push({ url, method: options.method || "GET", auth: (options.headers || {}).Authorization || null, body: null });
      const { status, body } = window.__driveShareRespond(url, options);
      return Promise.resolve(new Response(JSON.stringify(body), { status }));
    }
    if (url.includes("/drive_recipients")) {
      live.recipientsCalls.push({ url, method: options.method || "GET", body: options.body || null });
    }
    return realFetch(input, options);
  };
}
