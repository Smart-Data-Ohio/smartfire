// The server-rendered pages' only script: sign-in, joining, two-step sign-in, password
// confirmation, the sign-in link, first run and the public pages. No Turbo and no Stimulus; every
// form works without it except the two that submit themselves.
//
// Loaded blocking in <head>, so the appearance is applied before the first paint; everything that
// touches the page waits for DOMContentLoaded.
(() => {
  // The SPA's per-device appearance (frontend/src/lib/appearance.ts): a theme pinned on this
  // device, density and motion, remembered under one localStorage key. Only the pin is read for the
  // theme: the account's own theme is the server's (this page's data-theme), so one person's
  // theme never carries over to the next person who signs in on this browser.
  const root = document.documentElement;

  try {
    const saved = JSON.parse(localStorage.getItem("smartfire.appearance") || "null");

    if (saved && typeof saved === "object") {
      // A pin wins over the server's account theme, "system" included: the SPA removes the
      // attribute for it, so this page follows the OS just as the SPA does.
      const pin = saved.themeOverride;

      if (pin === "light" || pin === "dark") root.dataset.theme = pin;
      else if (pin === "system") delete root.dataset.theme;
      if (saved.motion === "reduce" || saved.motion === "full") root.dataset.motion = saved.motion;
      else if (saved.motion === "system") delete root.dataset.motion;
    }
  } catch {
    // Storage can be unavailable or hold something else; the OS setting decides.
  }

  // One submission per form: a second press while the page is leaving is dropped. The button is not
  // disabled, so its own name and value still go with the first submission.
  function guardSubmits() {
    document.addEventListener("submit", (event) => {
      const form = event.target;

      if (!(form instanceof HTMLFormElement)) return;

      if (form.dataset.submitting) {
        event.preventDefault();

        return;
      }

      form.dataset.submitting = "true";

      for (const button of form.querySelectorAll("button[type=submit], input[type=submit]")) {
        button.setAttribute("aria-busy", "true");
      }
    });

    // A page restored from the back/forward cache can be submitted again.
    window.addEventListener("pageshow", () => {
      for (const form of document.querySelectorAll("form[data-submitting]")) {
        delete form.dataset.submitting;

        for (const button of form.querySelectorAll("[aria-busy]")) {
          button.removeAttribute("aria-busy");
        }
      }
    });
  }

  // The sign-in link and a confirmed action carry on by themselves (classic's auto-submit).
  function autoSubmit() {
    for (const form of document.querySelectorAll(
      'form[data-controller~="auto-submit"], form[data-auto-submit]',
    )) {
      form.requestSubmit();
    }
  }

  // The avatar pickers show the chosen picture before it's uploaded.
  function avatarPreviews() {
    for (const input of document.querySelectorAll("input[type=file][data-avatar-input]")) {
      const picker = input.closest(".auth-avatar");
      const preview = picker?.querySelector("[data-avatar-preview]");
      const empty = picker?.querySelector("[data-avatar-empty]");
      let url = null;

      input.addEventListener("change", () => {
        const file = input.files?.[0];

        if (url) URL.revokeObjectURL(url);

        url = file ? URL.createObjectURL(file) : null;

        if (preview) {
          preview.hidden = url === null;

          if (url) preview.src = url;
        }

        if (empty) empty.hidden = url !== null;
      });
    }
  }

  // "Copy all" on the backup codes.
  function copyButtons() {
    for (const button of document.querySelectorAll("button[data-copy]")) {
      const label = button.querySelector("[data-copy-label]");
      const idle = label?.textContent ?? "";
      let timer = 0;

      button.addEventListener("click", async () => {
        try {
          await navigator.clipboard.writeText(button.dataset.copy ?? "");
        } catch {
          return;
        }

        button.dataset.copied = "true";

        if (label) label.textContent = "Copied";

        clearTimeout(timer);
        timer = window.setTimeout(() => {
          delete button.dataset.copied;

          if (label) label.textContent = idle;
        }, 2000);
      });
    }
  }

  // Translation lists are <details>. The field labels use .auth-translate; the unsupported-browser
  // page keeps the classic popup controller markup. Escape and a click elsewhere close them,
  // opening one closes the others, and the classic popup flips upward when it is near the bottom.
  function translationLists() {
    const lists = () =>
      document.querySelectorAll("details.auth-translate[open], details[data-controller~='popup'][open]");

    const isPopup = (list) => list.matches("[data-controller~='popup']");

    function orientPopup(list) {
      if (!isPopup(list)) return;

      const menu = list.querySelector("[data-popup-target='menu']");

      if (!menu) return;

      const rect = menu.getBoundingClientRect();
      const topClass = list.dataset.popupOrientationTopClass;

      if (topClass) list.classList.toggle(topClass, window.innerHeight - rect.bottom < 90);

      menu.style.setProperty("--max-width", `${window.innerWidth - rect.left}px`);
    }

    document.addEventListener("click", (event) => {
      for (const list of lists()) {
        if (!list.contains(event.target)) list.open = false;
      }
    });

    document.addEventListener("keydown", (event) => {
      if (event.key !== "Escape") return;

      for (const list of lists()) {
        list.open = false;
        list.querySelector("summary")?.focus();
      }
    });

    document.addEventListener(
      "toggle",
      (event) => {
        const opened = event.target;

        if (!(opened instanceof HTMLDetailsElement) || !opened.open) return;

        if (!opened.classList.contains("auth-translate") && !isPopup(opened)) return;

        orientPopup(opened);

        for (const list of lists()) {
          if (list !== opened) list.open = false;
        }
      },
      true,
    );
  }

  // The one service worker at scope "/", as classic's initializers/service_worker.js registers it:
  // the head names the worker for the effective UI (the stored choice, else SPA_DEFAULT), and
  // without the SPA it is the classic worker. Signing in or out is a full page load here, so the
  // next page registers the worker for the newly signed-in person. Re-registering a different
  // script swaps it in place; nothing unregisters. The test environment opts out.
  function registerServiceWorker() {
    if (!("serviceWorker" in navigator) || root.dataset.serviceWorker === "false") return;

    const url =
      document.querySelector('meta[name="service-worker-url"]')?.content || "/service-worker.js";

    navigator.serviceWorker.register(url, { scope: "/", updateViaCache: "none" }).catch(() => {});
  }

  window.addEventListener("load", registerServiceWorker);

  document.addEventListener("DOMContentLoaded", () => {
    guardSubmits();
    avatarPreviews();
    copyButtons();
    translationLists();
    autoSubmit();
  });
})();
