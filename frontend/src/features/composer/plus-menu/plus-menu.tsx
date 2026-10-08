import { Suspense, useEffect, useState } from "react";
import { useReducedMotion } from "../../../motion/reduced-motion.ts";
import { lazyForUpdate as lazy } from "../../../service-worker/lazy.ts";
import {
  ignoreModuleResourceLoadError,
  loadForUpdate,
} from "../../../service-worker/update-required.ts";
import { IconButton } from "../../../ui/icon-button.tsx";
import type { IconName } from "../../../ui/icons/icon.tsx";
import { Menu, MenuItem, MenuSeparator } from "../../../ui/menu.tsx";
import "./plus-menu.css";

/** One entry in the composer's + menu. */
export interface PlusAction {
  readonly id: string;
  readonly label: string;
  readonly icon: IconName;
  readonly shortcut?: readonly string[];
  readonly disabled?: boolean;
  /** Starts a new group (a separator before it). */
  readonly groupStart?: boolean;
  readonly onSelect: () => void;
}

export interface PlusMenuProps {
  readonly actions: readonly PlusAction[];
}

const TRIGGER_LABEL = "Attach and more";

/** Loaded on idle after the composer mounts, so the first open is already liquid. */
const loadGooey = () => loadForUpdate(() => import("./gooey-plus-menu.tsx"));

const GooeyPlusMenu = lazy(loadGooey);

/** Cheap devices (few cores, little memory, data saver) get the plain dropdown. */
function lowPower(): boolean {
  const cores = navigator.hardwareConcurrency;
  const memory = "deviceMemory" in navigator ? Number(navigator.deviceMemory) : 8;

  return (
    (cores > 0 && cores <= 2) ||
    memory < 2 ||
    window.matchMedia("(prefers-reduced-data: reduce)").matches
  );
}

/** Runs `work` when the main thread is idle (a short timeout where idle callbacks don't exist). */
function whenIdle(work: () => void): () => void {
  if ("requestIdleCallback" in window) {
    const handle = window.requestIdleCallback(work, { timeout: 2000 });

    return () => window.cancelIdleCallback(handle);
  }

  const handle = globalThis.setTimeout(work, 200);

  return () => globalThis.clearTimeout(handle);
}

/** The plain menu: the design system's dropdown (menu-dropdown recipe). */
export function PlainPlusMenu({ actions }: PlusMenuProps) {
  return (
    <Menu
      placement="top-start"
      label={TRIGGER_LABEL}
      trigger={(props) => (
        <IconButton
          {...props}
          icon="plus"
          label={TRIGGER_LABEL}
          size="sm"
          className="composer-plus"
          onMouseDown={(event) => event.preventDefault()}
        />
      )}
    >
      {actions.flatMap((action) => {
        const item = (
          <MenuItem
            key={action.id}
            icon={action.icon}
            shortcut={action.shortcut ?? []}
            disabled={action.disabled === true}
            onSelect={action.onSelect}
          >
            {action.label}
          </MenuItem>
        );

        return action.groupStart === true
          ? [<MenuSeparator key={`${action.id}-separator`} />, item]
          : [item];
      })}
    </Menu>
  );
}

/**
 * The composer's + button. It opens a menu that splits out of the button as liquid
 * (Jakub Antalik's liquid-gooey, lazy-loaded, one instance per composer); under reduced motion,
 * on low-power devices and until the chunk arrives, it is the plain dropdown.
 */
export function PlusMenu({ actions }: PlusMenuProps) {
  const reduced = useReducedMotion();
  const [plain] = useState(lowPower);

  useEffect(() => {
    if (reduced || plain) {
      return;
    }

    return whenIdle(() => void loadGooey().catch(ignoreModuleResourceLoadError));
  }, [reduced, plain]);

  if (reduced || plain) {
    return <PlainPlusMenu actions={actions} />;
  }

  return (
    <Suspense fallback={<PlainPlusMenu actions={actions} />}>
      <GooeyPlusMenu actions={actions} label={TRIGGER_LABEL} />
    </Suspense>
  );
}
