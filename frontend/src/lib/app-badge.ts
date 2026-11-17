/** A browser may deny badging without preventing sign-out or an account change. */
export async function clearAppBadge(): Promise<void> {
  try {
    await navigator.clearAppBadge?.();
  } catch {
    // Badging is optional.
  }
}
