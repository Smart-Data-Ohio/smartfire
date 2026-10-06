/**
 * Whether the person is driving with the keyboard or a pointer right now. A popup or dialog that
 * closes hands focus back to its message row only for keyboard use; after a click, focus parked
 * on the row would keep its hover bar showing.
 */
let keyboard = false;

let listening = false;

function listen(): void {
  if (listening) {
    return;
  }

  listening = true;
  document.addEventListener(
    "keydown",
    () => {
      keyboard = true;
    },
    true,
  );
  document.addEventListener(
    "pointerdown",
    () => {
      keyboard = false;
    },
    true,
  );
}

/** Starts tracking (idempotent); call from anything that will ask later. */
export function trackModality(): void {
  listen();
}

export function usingKeyboard(): boolean {
  listen();

  return keyboard;
}
