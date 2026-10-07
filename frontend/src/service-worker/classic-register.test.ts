import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { runInNewContext } from "node:vm";
import { describe, expect, it, vi } from "vitest";

const source = readFileSync(
  resolve("../crates/assets/overrides/smartfire/initializers/service_worker.js"),
  "utf8",
);

function startup(controller?: { scriptURL: string }) {
  const doc = document.implementation.createHTMLDocument("Smartfire");
  const base = doc.createElement("base");

  base.href = "https://smartfire.test/rooms/1";
  doc.head.append(base);
  doc.documentElement.dataset.serviceWorker = "true";

  const meta = doc.createElement("meta");

  meta.name = "service-worker-url";
  meta.content = "/service-worker.js";
  doc.head.append(meta);

  const page = Object.assign(new EventTarget(), {
    location: new URL(base.href),
    notificationsPreviouslyReady: false,
    Notification: {},
    matchMedia: () => ({ matches: false }),
  });

  const register = vi.fn(async () => undefined);
  const serviceWorker = { register, controller };

  runInNewContext(source, {
    document: doc,
    window: page,
    navigator: { serviceWorker },
    URL,
    Promise,
  });

  return { doc, page, meta, register, serviceWorker };
}

function image(doc: Document, src: string, decode: () => Promise<void>) {
  const element = doc.createElement("img");

  element.src = src;
  element.decode = decode;
  doc.body.append(element);

  return element;
}

const settled = () => new Promise<void>((resolve) => setTimeout(resolve, 0));

describe("classic service worker startup", () => {
  it("waits for hidden public bell images before replacing the root worker", async () => {
    const f = startup();
    const ready = Promise.withResolvers<void>();
    const decode = vi.fn(() => ready.promise);
    const bell = image(f.doc, "/assets/notification-bell-alert-01234567.svg", decode);

    bell.hidden = true;
    f.page.dispatchEvent(new Event("load"));
    await settled();
    expect(decode).toHaveBeenCalledOnce();
    expect(f.register).not.toHaveBeenCalled();
    ready.resolve();
    await settled();
    expect(f.register).toHaveBeenCalledExactlyOnceWith("/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
  });

  it.each(["throw", "reject"])(
    "still registers when a public image decode reports %s",
    async (failure) => {
      const f = startup();

      image(f.doc, "/assets/notification-bell-alert-01234567.svg", () => {
        const error = new Error("Image decoding failed");

        if (failure === "throw") throw error;

        return Promise.reject(error);
      });
      f.page.dispatchEvent(new Event("load"));
      await settled();
      expect(f.register).toHaveBeenCalledOnce();
    },
  );

  it("excludes private, foreign, unhashed, lazy and responsive images", async () => {
    const f = startup();
    const decode = vi.fn(() => new Promise<void>(() => undefined));

    for (const src of [
      "/rails/active_storage/blobs/redirect/secret/avatar.png",
      "https://elsewhere.test/assets/avatar-01234567.svg",
      "/assets/avatar.svg",
      "http://[invalid/assets/avatar-01234567.svg",
    ]) {
      image(f.doc, src, decode);
    }

    image(f.doc, "/assets/lazy-01234567.svg", decode).loading = "lazy";
    image(f.doc, "/assets/responsive-01234567.svg", decode).srcset = "/private/avatar.png 2x";

    const picture = f.doc.createElement("picture");

    picture.append(image(f.doc, "/assets/picture-01234567.svg", decode));
    f.doc.body.append(picture);
    f.page.dispatchEvent(new Event("load"));
    await settled();
    expect(decode).not.toHaveBeenCalled();
    expect(f.register).toHaveBeenCalledOnce();
  });

  it("registers on a page without public images", async () => {
    const f = startup();

    f.page.dispatchEvent(new Event("load"));
    await settled();
    expect(f.register).toHaveBeenCalledOnce();
  });

  it("drops a changed worker URL until its Turbo visit completes its own image barrier", async () => {
    const f = startup();
    const ready = Promise.withResolvers<void>();
    const nextReady = Promise.withResolvers<void>();

    const bell = image(f.doc, "/assets/bell-01234567.svg", () => ready.promise);

    f.page.dispatchEvent(new Event("load"));
    await settled();
    f.meta.content = "/app/service-worker.js";
    ready.resolve();
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    bell.decode = () => nextReady.promise;
    f.doc.dispatchEvent(new Event("turbo:load"));
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    nextReady.resolve();
    await settled();
    expect(f.register).toHaveBeenCalledExactlyOnceWith("/app/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
  });

  it.each(["opt-out", "preview"])(
    "invalidates pending work on a %s lifecycle event",
    async (disabled) => {
      const f = startup();
      const ready = Promise.withResolvers<void>();

      image(f.doc, "/assets/bell-01234567.svg", () => ready.promise);
      f.page.dispatchEvent(new Event("load"));
      await settled();

      if (disabled === "opt-out") f.doc.documentElement.dataset.serviceWorker = "false";
      else f.doc.documentElement.setAttribute("data-turbo-preview", "");

      f.doc.dispatchEvent(new Event("turbo:load"));
      f.doc.documentElement.dataset.serviceWorker = "true";
      f.doc.documentElement.removeAttribute("data-turbo-preview");
      ready.resolve();
      await settled();
      expect(f.register).not.toHaveBeenCalled();
    },
  );

  it("lets the newer Turbo body reconcile while dropping the older body's pending decode", async () => {
    const f = startup();
    const ready = Promise.withResolvers<void>();

    image(f.doc, "/assets/bell-01234567.svg", () => ready.promise);
    f.page.dispatchEvent(new Event("load"));
    await settled();
    f.doc.body.replaceWith(f.doc.createElement("body"));
    f.meta.content = "/app/service-worker.js";
    f.doc.dispatchEvent(new Event("turbo:load"));
    await settled();
    ready.resolve();
    await settled();
    expect(f.register).toHaveBeenCalledExactlyOnceWith("/app/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
  });

  it("rechecks opt-in and body identity after images settle even without another lifecycle event", async () => {
    for (const change of ["opt-out", "body"]) {
      const f = startup();
      const ready = Promise.withResolvers<void>();

      image(f.doc, "/assets/bell-01234567.svg", () => ready.promise);
      f.page.dispatchEvent(new Event("load"));
      await settled();

      if (change === "opt-out") f.doc.documentElement.dataset.serviceWorker = "false";
      else f.doc.body.replaceWith(f.doc.createElement("body"));

      ready.resolve();
      await settled();
      expect(f.register).not.toHaveBeenCalled();
    }
  });
});

type StartupDetail = {
  completion?: Promise<void>;
  completions?: Promise<void>[];
};

interface NotificationStartupController {
  connect(): void;
}

interface FrameStartupController {
  load(event: { params: { url: string }; detail: StartupDetail }): void;
  disconnect(): void;
}

interface ExistingPushSubscription {
  endpoint: string;
}

interface ControllerGlobals {
  Controller: new () => { element: HTMLElement };
  nextEventLoopTick(): Promise<void>;
  window?: ReturnType<typeof startup>["page"];
  document?: Document;
  navigator?: {
    serviceWorker: {
      getRegistration(): Promise<{
        pushManager: { getSubscription(): Promise<ExistingPushSubscription | null> };
      }>;
    };
  };
  Notification?: { permission: string };
  pageIsTurboPreview?(): boolean;
  getCookie?(): boolean;
}

function classicController<T>(name: string, globals: ControllerGlobals) {
  const code = readFileSync(
    resolve(`../crates/assets/overrides/smartfire/controllers/${name}_controller.js`),
    "utf8",
  )
    .replace(/^import .*$/gm, "")
    .replace("export default class", "class TestController");

  let controller: T | undefined;

  runInNewContext(`${code}\ncapture(new TestController())`, {
    ...globals,
    Promise,
    MutationObserver,
    capture: (value: T) => {
      controller = value;
    },
  });

  if (!controller) throw new Error("Controller did not initialize");

  return controller;
}

function eventLoop() {
  const pending: ReturnType<typeof Promise.withResolvers<void>>[] = [];

  return {
    nextEventLoopTick() {
      const tick = Promise.withResolvers<void>();
      pending.push(tick);

      return tick.promise;
    },
    advance() {
      const tick = pending.shift();

      if (!tick) throw new Error("No queued task");
      tick.resolve();
    },
  };
}

function notifications(
  f: ReturnType<typeof startup>,
  tasks: ReturnType<typeof eventLoop>,
  onReady: (detail: StartupDetail) => void = () => undefined,
  getSubscription: () => Promise<ExistingPushSubscription | null> = async () => ({
    endpoint: "existing-push",
  }),
  permission = "granted",
) {
  const element = f.doc.createElement("span");
  const bell = f.doc.createElement("button");
  element.dataset.controller = "notifications";
  element.append(bell);
  f.doc.body.append(element);

  class Controller {
    element = element;
    bellTarget = bell;
    attentionClass = "attention";

    dispatch(name: string, { detail }: { detail: StartupDetail }) {
      element.dispatchEvent(new CustomEvent(`notifications:${name}`, { bubbles: true, detail }));

      if (name === "ready") onReady(detail);
    }
  }

  const controller = classicController<NotificationStartupController>("notifications", {
    Controller,
    window: f.page,
    document: f.doc,
    navigator: {
      serviceWorker: {
        getRegistration: async () => ({ pushManager: { getSubscription } }),
      },
    },
    Notification: { permission },
    pageIsTurboPreview: () => f.doc.documentElement.hasAttribute("data-turbo-preview"),
    nextEventLoopTick: tasks.nextEventLoopTick,
    getCookie: () => true,
  });

  return { controller, element };
}

function notificationFrame(
  f: ReturnType<typeof startup>,
  tasks: ReturnType<typeof eventLoop>,
  response: () => Promise<void>,
) {
  let loaded: Promise<void> = Promise.resolve();
  const requests: string[] = [];

  const element = Object.assign(f.doc.createElement("turbo-frame"), {
    reload() {
      const src = element.getAttribute("src");
      element.removeAttribute("src");

      if (src) start(src);
    },
  });

  const start = (url: string) => {
    element.setAttribute("src", url);
    requests.push(url);
    loaded = response();
  };

  Object.defineProperties(element, {
    src: { set: start },
    loaded: { get: () => loaded },
  });
  f.doc.body.append(element);

  class Controller {
    element = element;
  }

  const controller = classicController<FrameStartupController>("turbo_frame", {
    Controller,
    nextEventLoopTick: tasks.nextEventLoopTick,
  });

  return { element, controller, requests };
}

const switchingFromSpa = { scriptURL: "https://smartfire.test/app/service-worker.js" };

describe("classic worker handoff after notification startup", () => {
  it("waits for a lazily connected notification controller, queued frame render, and its new images", async () => {
    const f = startup(switchingFromSpa);
    const tasks = eventLoop();
    const subscription = Promise.withResolvers<ExistingPushSubscription | null>();
    const rendered = Promise.withResolvers<void>();
    const decoded = Promise.withResolvers<void>();
    const frame = notificationFrame(f, tasks, () => rendered.promise);

    const notice = notifications(
      f,
      tasks,
      (detail) => {
        frame.controller.load({ params: { url: "/rooms/1/involvement" }, detail });
      },
      () => subscription.promise,
    );

    f.page.dispatchEvent(new Event("load"));
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    notice.controller.connect();
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    subscription.resolve({ endpoint: "existing-push" });
    await settled();
    tasks.advance();
    await settled();
    expect(frame.requests).toEqual([]);
    expect(f.register).not.toHaveBeenCalled();
    tasks.advance();
    await settled();
    expect(frame.requests).toEqual(["/rooms/1/involvement"]);
    expect(f.register).not.toHaveBeenCalled();
    frame.element.append(
      image(f.doc, "/assets/notification-bell-mentions-01234567.svg", () => decoded.promise),
    );
    rendered.resolve();
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    decoded.resolve();
    await settled();
    expect(f.register).toHaveBeenCalledOnce();
  });

  it.each(["cold", "same-script"])(
    "does not await notification startup on %s registration",
    async (kind) => {
      const f = startup(
        kind === "cold" ? undefined : { scriptURL: "https://smartfire.test/service-worker.js" },
      );

      notifications(f, eventLoop());
      f.page.dispatchEvent(new Event("load"));
      await settled();
      expect(f.register).toHaveBeenCalledOnce();
    },
  );

  it.each(["denied", "no-subscription", "rejected"])(
    "settles notification startup when push is %s",
    async (kind) => {
      const f = startup(switchingFromSpa);
      const tasks = eventLoop();

      const getSubscription =
        kind === "rejected"
          ? () => Promise.reject(new Error("Push service unavailable"))
          : async () => (kind === "no-subscription" ? null : { endpoint: "existing-push" });

      const notice = notifications(
        f,
        tasks,
        () => {
          throw new Error("Disabled notifications must not load a frame");
        },
        getSubscription,
        kind === "denied" ? "denied" : "granted",
      );

      f.page.dispatchEvent(new Event("load"));
      notice.controller.connect();
      await settled();
      expect(f.register).toHaveBeenCalledOnce();
    },
  );

  it("ignores a pending old notification startup after a newer Turbo body and selected worker", async () => {
    const f = startup(switchingFromSpa);
    const tasks = eventLoop();
    const subscription = Promise.withResolvers<ExistingPushSubscription | null>();

    const notice = notifications(
      f,
      tasks,
      () => undefined,
      () => subscription.promise,
    );

    f.page.dispatchEvent(new Event("load"));
    notice.controller.connect();
    await settled();
    f.doc.body.replaceWith(f.doc.createElement("body"));
    f.meta.content = "/app/service-worker.js";
    f.doc.dispatchEvent(new Event("turbo:load"));
    await settled();
    expect(f.register).toHaveBeenCalledExactlyOnceWith("/app/service-worker.js", {
      scope: "/",
      updateViaCache: "none",
    });
    subscription.resolve(null);
    await settled();
    expect(f.register).toHaveBeenCalledOnce();
  });

  it("waits for notification startup already published before the load event", async () => {
    const f = startup(switchingFromSpa);
    const tasks = eventLoop();
    const subscription = Promise.withResolvers<ExistingPushSubscription | null>();

    const notice = notifications(
      f,
      tasks,
      () => undefined,
      () => subscription.promise,
    );

    notice.controller.connect();
    f.page.dispatchEvent(new Event("load"));
    await settled();
    expect(f.register).not.toHaveBeenCalled();
    subscription.resolve(null);
    await settled();
    expect(f.register).toHaveBeenCalledOnce();
  });
});

describe("notification frame startup completion", () => {
  it("follows a direct native reload when its original loaded promise remains pending", async () => {
    const f = startup();
    const tasks = eventLoop();
    const initial = Promise.withResolvers<void>();
    const replacement = Promise.withResolvers<void>();
    let response = initial.promise;
    const frame = notificationFrame(f, tasks, () => response);
    const completions: Promise<void>[] = [];
    frame.controller.load({ params: { url: "/rooms/1/involvement" }, detail: { completions } });
    const done = vi.fn();
    Promise.all(completions).then(done);
    tasks.advance();
    await settled();
    response = replacement.promise;
    frame.element.reload();
    await settled();
    expect(done).not.toHaveBeenCalled();
    replacement.resolve();
    await settled();
    expect(done).toHaveBeenCalledOnce();
    expect(frame.requests).toEqual(["/rooms/1/involvement", "/rooms/1/involvement"]);
  });

  it("follows a second load and ignores the first request resolving late", async () => {
    const f = startup();
    const tasks = eventLoop();
    const first = Promise.withResolvers<void>();
    const second = Promise.withResolvers<void>();
    let response = first.promise;
    const frame = notificationFrame(f, tasks, () => response);
    const completions: Promise<void>[] = [];
    frame.controller.load({ params: { url: "/rooms/1/involvement" }, detail: { completions } });
    tasks.advance();
    await settled();
    response = second.promise;
    frame.controller.load({ params: { url: "/rooms/2/involvement" }, detail: { completions } });
    const done = vi.fn();
    Promise.all(completions).then(done);
    tasks.advance();
    await settled();
    first.resolve();
    await settled();
    expect(done).not.toHaveBeenCalled();
    second.resolve();
    await settled();
    expect(done).toHaveBeenCalledOnce();
  });

  it.each(["queued", "fetching"])("settles on disconnect while %s", async (stage) => {
    const f = startup();
    const tasks = eventLoop();
    const frame = notificationFrame(f, tasks, () => new Promise<void>(() => undefined));
    const completions: Promise<void>[] = [];
    frame.controller.load({ params: { url: "/rooms/1/involvement" }, detail: { completions } });
    const done = vi.fn();
    Promise.all(completions).then(done);

    if (stage === "fetching") {
      tasks.advance();
      await settled();
    }

    frame.controller.disconnect();
    await settled();
    expect(done).toHaveBeenCalledOnce();

    if (stage === "queued") {
      tasks.advance();
      await settled();
      expect(frame.requests).toEqual([]);
    }
  });
});
