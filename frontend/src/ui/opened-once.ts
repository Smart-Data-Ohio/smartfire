import { useState } from "react";

/**
 * Whether `open` has ever been true: lets a lazily loaded dialog or panel stay unloaded until it's
 * first wanted, then stay mounted so its closing transition still plays.
 */
export function useOpenedOnce(open: boolean): boolean {
  const [opened, setOpened] = useState(open);

  if (open && !opened) {
    setOpened(true);
  }

  return opened || open;
}
