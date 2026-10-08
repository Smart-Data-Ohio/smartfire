import {
  type KeyboardEvent as ReactKeyboardEvent,
  type PointerEvent as ReactPointerEvent,
  Suspense,
  useEffect,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { usePresence } from "../../motion/presence.ts";
import { lazyForUpdate as lazy } from "../../service-worker/lazy.ts";
import { loadForUpdate } from "../../service-worker/update-required.ts";
import { useStore } from "../../store/store.ts";
import { type PaneChrome, PaneChromeContext, PaneFrame } from "./pane-frame.tsx";
import { PANE_TITLES, type RightPaneView, viewKey } from "./pane-selection.ts";
import { PaneListSkeleton } from "./pane-states.tsx";
import { useOpenPane } from "./pane-store.ts";
import { usePaneNavigation, usePhoneLayout } from "./use-right-pane.ts";
import "./panes.css";

const WIDTH_KEY = "smartfire.rightPaneWidth";

const MIN_WIDTH = 320;

const MAX_WIDTH = 600;

const DEFAULT_WIDTH = 400;

/** A remembered width, clamped to what the column allows. */
export function clampPaneWidth(width: number): number {
  return Number.isFinite(width)
    ? Math.round(Math.min(MAX_WIDTH, Math.max(MIN_WIDTH, width)))
    : DEFAULT_WIDTH;
}

function readWidth(): number {
  try {
    const stored = localStorage.getItem(WIDTH_KEY);

    return stored === null ? DEFAULT_WIDTH : clampPaneWidth(Number(stored));
  } catch {
    return DEFAULT_WIDTH;
  }
}

function writeWidth(width: number): void {
  try {
    localStorage.setItem(WIDTH_KEY, String(width));
  } catch {
    // Without storage the width lasts until reload.
  }
}

/** A field with something written in it, or anything inside a composer holding text or files. */
function holdsWriting(target: Element): boolean {
  if (target.closest(".composer[data-holding]") !== null) {
    return true;
  }

  if (target instanceof HTMLTextAreaElement) {
    return target.value !== "";
  }

  const editable = target.closest('[contenteditable]:not([contenteditable="false"])');

  return editable !== null && (editable.textContent ?? "").trim() !== "";
}

/**
 * Esc belongs to whatever has focus first: menus, dialogs, popovers, a search being cleared, an
 * IME choosing characters, and anything with writing in it (a composer's text or uploads, an
 * edit), so a stray Esc can't throw a draft's files away. An empty composer passes it on: the
 * pane focuses its composer on open, and Esc still closes it.
 */
function escapeIsTaken(event: KeyboardEvent): boolean {
  if (event.defaultPrevented || event.isComposing) {
    return true;
  }

  const target = event.target instanceof Element ? event.target : null;

  return (
    (target !== null && holdsWriting(target)) ||
    target?.closest('[role="menu"], dialog, [popover], [role="listbox"]') != null ||
    document.querySelector("dialog[open]") !== null
  );
}

// The panes load on demand: the first open fetches its chunk, and an idle moment after the right
// pane first mounts fetches the rest.
const loadThreadPane = () => loadForUpdate(() => import("../threads/thread-pane.tsx"));

const loadNewThreadPane = () => loadForUpdate(() => import("../threads/new-thread-pane.tsx"));

const loadThreadsPane = () => loadForUpdate(() => import("../threads/threads-pane.tsx"));

const loadMembersPane = () => loadForUpdate(() => import("./members-pane.tsx"));

const loadPinsPane = () => loadForUpdate(() => import("./pins-pane.tsx"));

const loadFilesPane = () => loadForUpdate(() => import("./files-pane.tsx"));

const loadStagePane = () => loadForUpdate(() => import("../huddle/stage-pane.tsx"));

const loadBoardAutomationsPane = () =>
  loadForUpdate(() => import("../boards/automations-pane.tsx"));

const ThreadPane = lazy(async () => {
  const module = await loadThreadPane();

  return { default: module.ThreadPane };
});

const NewThreadPane = lazy(async () => {
  const module = await loadNewThreadPane();

  return { default: module.NewThreadPane };
});

const ThreadsPane = lazy(async () => {
  const module = await loadThreadsPane();

  return { default: module.ThreadsPane };
});

const MembersPane = lazy(async () => {
  const module = await loadMembersPane();

  return { default: module.MembersPane };
});

const PinsPane = lazy(async () => {
  const module = await loadPinsPane();

  return { default: module.PinsPane };
});

const FilesPane = lazy(async () => {
  const module = await loadFilesPane();

  return { default: module.FilesPane };
});

const StagePane = lazy(async () => {
  const module = await loadStagePane();

  return { default: module.StagePane };
});

const BoardAutomationsPane = lazy(async () => {
  const module = await loadBoardAutomationsPane();

  return { default: module.BoardAutomationsPane };
});

let preloaded = false;

/** Fetches every pane's chunk once the page is idle, so later opens don't wait on the network. */
function preloadPanes(): void {
  if (preloaded) {
    return;
  }

  preloaded = true;

  const load = () => {
    for (const loader of [
      loadThreadPane,
      loadNewThreadPane,
      loadThreadsPane,
      loadMembersPane,
      loadPinsPane,
      loadFilesPane,
      loadStagePane,
      loadBoardAutomationsPane,
    ]) {
      void loader().catch(() => undefined);
    }
  };

  if ("requestIdleCallback" in window) {
    window.requestIdleCallback(load, { timeout: 4000 });
  } else {
    setTimeout(load, 2000);
  }
}

function fallbackTitle(view: RightPaneView): string {
  switch (view.kind) {
    case "pane":
      return PANE_TITLES[view.pane];
    case "thread":
      return "Thread";
    case "new-thread":
      return "New thread";
  }
}

function PaneFallback({ view }: { readonly view: RightPaneView }) {
  const title = fallbackTitle(view);

  return (
    <PaneFrame title={title}>
      <PaneListSkeleton rows={4} />
    </PaneFrame>
  );
}

function PaneBody({ roomId, view }: { readonly roomId: number; readonly view: RightPaneView }) {
  switch (view.kind) {
    case "thread":
      return <ThreadPane roomId={roomId} threadId={view.threadId} />;
    case "new-thread":
      return <NewThreadPane key={view.parentId} roomId={roomId} parentId={view.parentId} />;
    case "pane":
      switch (view.pane) {
        case "members":
          return <MembersPane roomId={roomId} />;
        case "pins":
          return <PinsPane roomId={roomId} />;
        case "files":
          return <FilesPane roomId={roomId} />;
        case "threads":
          return <ThreadsPane roomId={roomId} />;
        case "stage":
          return <StagePane roomId={roomId} />;
        case "automations":
          return <BoardAutomationsPane roomId={roomId} />;
      }
  }
}

/** Moves focus into a pane that just opened: its search field or composer, else the pane. */
function focusInto(pane: HTMLElement, phone: boolean): void {
  const preferred = pane.querySelector<HTMLElement>(
    phone ? "[data-pane-autofocus]" : "[data-pane-autofocus], .composer-input",
  );

  (preferred ?? pane).focus({ preventScroll: true });
}

/**
 * Beside the conversation: the open thread (from the URL) or the open side pane (members, pins,
 * files, threads). A 400 px column on wide screens (drag its edge to resize), a sheet over the
 * conversation below 1100 px, and a full-screen page on phones. It reveals with the panel-reveal
 * recipe (a page slide on phones), stays mounted through its exit, closes on Esc, and hands focus
 * back to whatever opened it.
 */
export function RightPane({ roomId }: { readonly roomId: number }) {
  const navigation = usePaneNavigation(roomId);
  const { view } = navigation;
  const phone = usePhoneLayout();
  const underPane = useOpenPane(roomId);
  const headingId = useId();
  const presence = usePresence<HTMLElement>(view !== null);
  const [shown, setShown] = useState<RightPaneView | null>(view);
  const [entered, setEntered] = useState(false);
  const [width, setWidth] = useState(readWidth);
  const openerRef = useRef<Element | null>(null);

  const roomName = useStore((state) => {
    const detail = state.rooms[roomId]?.detail;

    return detail?.displayName ?? state.sidebar.rows[roomId]?.displayName ?? "the conversation";
  });

  const kind = useStore(
    (state) => state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind,
  );

  // Keep the last view while the pane plays its exit.
  if (view !== null && (shown === null || viewKey(shown) !== viewKey(view))) {
    setShown(view);
  }

  if (!presence.mounted && shown !== null) {
    setShown(null);
  }

  const open = view !== null;

  // The enter starts from the closed style: flip to open once that style has been laid out.
  useLayoutEffect(() => {
    const element = presence.ref.current;

    if (!presence.mounted || element === null) {
      setEntered(false);

      return;
    }

    if (open && !entered) {
      void element.offsetWidth;
      setEntered(true);
    }
  }, [presence.mounted, presence.ref, open, entered]);

  // Focus goes in when the pane opens or changes, and back to the opener when it closes.
  const shownKey = shown === null ? null : viewKey(shown);

  useEffect(() => {
    const element = presence.ref.current;

    if (!open || element === null || shownKey === null) {
      return;
    }

    if (!element.contains(document.activeElement)) {
      openerRef.current ??= document.activeElement;
    }

    const frame = requestAnimationFrame(() => focusInto(element, phone));

    return () => cancelAnimationFrame(frame);
  }, [open, shownKey, phone, presence.ref]);

  useEffect(() => {
    if (open) {
      return;
    }

    const opener = openerRef.current;

    openerRef.current = null;

    const focusLost =
      document.activeElement === document.body ||
      presence.ref.current?.contains(document.activeElement) === true;

    if (opener instanceof HTMLElement && opener.isConnected && focusLost) {
      opener.focus({ preventScroll: true });
    }
  }, [open, presence.ref]);

  useEffect(preloadPanes, []);

  const { closeTop } = navigation;

  useEffect(() => {
    if (!open) {
      return;
    }

    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || escapeIsTaken(event)) {
        return;
      }

      event.preventDefault();
      closeTop();
    };

    window.addEventListener("keydown", onKeyDown);

    return () => window.removeEventListener("keydown", onKeyDown);
  }, [open, closeTop]);

  if (!presence.mounted || shown === null) {
    return null;
  }

  const overThread = shown.kind !== "pane" && underPane !== null;

  const backLabel = overThread
    ? `Back to ${PANE_TITLES[underPane].toLowerCase()}`
    : `Back to ${kind === "direct" ? roomName : `#${roomName}`}`;

  const chrome: PaneChrome = {
    onBack: phone || overThread ? closeTop : null,
    backLabel,
    onClose: navigation.closeAll,
    phone,
    headingId,
  };

  const startResize = (event: ReactPointerEvent<HTMLDivElement>) => {
    const startX = event.clientX;
    const startWidth = width;
    const handle = event.currentTarget;

    handle.setPointerCapture(event.pointerId);

    const onMove = (move: PointerEvent) =>
      setWidth(clampPaneWidth(startWidth + startX - move.clientX));

    const onUp = (up: PointerEvent) => {
      const next = clampPaneWidth(startWidth + startX - up.clientX);

      setWidth(next);
      writeWidth(next);
      handle.removeEventListener("pointermove", onMove);
      handle.removeEventListener("pointerup", onUp);
      handle.removeEventListener("pointercancel", onUp);
    };

    handle.addEventListener("pointermove", onMove);
    handle.addEventListener("pointerup", onUp);
    handle.addEventListener("pointercancel", onUp);
  };

  const resizeWithKeys = (event: ReactKeyboardEvent<HTMLDivElement>) => {
    const step = event.shiftKey ? 64 : 16;

    const moves = new Map([
      ["ArrowLeft", width + step],
      ["ArrowRight", width - step],
      ["Home", MAX_WIDTH],
      ["End", MIN_WIDTH],
    ]);

    const next = moves.get(event.key);

    if (next !== undefined) {
      event.preventDefault();
      setWidth(clampPaneWidth(next));
      writeWidth(clampPaneWidth(next));
    }
  };

  return (
    <aside
      ref={presence.ref}
      className={`right-pane ${phone ? "t-page" : "t-panel-slide"}`}
      data-page-id={phone ? "2" : undefined}
      data-open={open && entered}
      data-state={presence.state}
      aria-labelledby={headingId}
      tabIndex={-1}
      inert={!open}
      style={{ "--right-pane-width": `${width}px` }}
    >
      {phone ? null : (
        // biome-ignore lint/a11y/useSemanticElements: a focusable separator is the WAI-ARIA window splitter; <hr> can't take focus or keys
        <div
          className="right-pane-resize"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize the side pane"
          aria-valuemin={MIN_WIDTH}
          aria-valuemax={MAX_WIDTH}
          aria-valuenow={width}
          tabIndex={0}
          onPointerDown={startResize}
          onKeyDown={resizeWithKeys}
          onDoubleClick={() => {
            setWidth(DEFAULT_WIDTH);
            writeWidth(DEFAULT_WIDTH);
          }}
        />
      )}
      <PaneChromeContext value={chrome}>
        <div key={viewKey(shown)} className="right-pane-view">
          <Suspense fallback={<PaneFallback view={shown} />}>
            <PaneBody roomId={roomId} view={shown} />
          </Suspense>
        </div>
      </PaneChromeContext>
    </aside>
  );
}
