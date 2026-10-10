import { store } from "../store/store.ts";

const SELECTOR = 'style[data-turbo-track="reload"]';

let saved: string | null = null;

let preview: string | null = null;

function render(css: string | null): void {
  const existing = document.head.querySelector(SELECTOR);

  if (css === null || css === "") {
    existing?.remove();

    return;
  }

  const style = existing ?? document.createElement("style");

  style.setAttribute("data-turbo-track", "reload");
  // Match shell.rs: raw CSS, with CSS escapes for HTML's end-tag delimiter. Classic's inline
  // style-src permits this element. Unlayered workspace CSS beats base layers; appearance's
  // explicit choices use the root CSSOM, above workspace defaults.
  style.textContent = css.replace(/<\/style(?=[\t\n\f\r />])/gi, (tag) => `\\3c ${tag.slice(1)}`);

  if (existing === null) document.head.append(style);
}

/** Boot, saves and the account's live channel all use the shell's style element. */
export function applyWorkspaceStyles(css: string | null): void {
  saved = css;
  render(preview ?? saved);
}

/** Preview the actual page; cleanup restores the latest saved styles, including live changes. */
export function previewWorkspaceStyles(css: string): () => void {
  preview = css;
  render(css);

  return () => {
    preview = null;
    render(saved);
  };
}

export function followWorkspaceStyles(): () => void {
  const show = () => {
    const boot = store.getState().boot;

    if (boot !== null) applyWorkspaceStyles(boot.customStyles);
  };

  show();

  return store.subscribe((state, previous) => {
    if (state.boot?.customStyles !== previous.boot?.customStyles) show();
  });
}
