/**
 * A tiny module store for toasts, so anything can raise one (`toast({ title })`) without a
 * provider; the Toaster subscribes with useSyncExternalStore.
 */
export type ToastTone = "neutral" | "success" | "danger";

export interface ToastAction {
  readonly label: string;
  readonly onClick: () => void;
}

export interface ToastInput {
  readonly title: string;
  readonly description?: string;
  readonly tone?: ToastTone;
  readonly action?: ToastAction;
}

export interface ToastRecord extends ToastInput {
  readonly id: number;
  readonly closing: boolean;
}

let toasts: readonly ToastRecord[] = [];

let nextId = 1;

const listeners = new Set<() => void>();

function emit(next: readonly ToastRecord[]): void {
  toasts = next;

  for (const listener of listeners) {
    listener();
  }
}

export function subscribeToasts(listener: () => void): () => void {
  listeners.add(listener);

  return () => {
    listeners.delete(listener);
  };
}

export function toastSnapshot(): readonly ToastRecord[] {
  return toasts;
}

/** Shows a toast; returns its id. */
export function toast(input: ToastInput): number {
  const id = nextId;

  nextId += 1;
  emit([...toasts, { ...input, id, closing: false }]);

  return id;
}

/** Starts a toast's exit; the Toaster removes it once the exit has played. */
export function dismissToast(id: number): void {
  emit(toasts.map((record) => (record.id === id ? { ...record, closing: true } : record)));
}

export function removeToast(id: number): void {
  emit(toasts.filter((record) => record.id !== id));
}
