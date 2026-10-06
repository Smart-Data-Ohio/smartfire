import { useEffect, useLayoutEffect, useRef, useState, useSyncExternalStore } from "react";
import { hidePopover, showPopover, supportsPopover } from "../lib/popover.ts";
import { readDurationMs } from "../motion/durations.ts";
import { afterExit } from "../motion/presence.ts";
import { Button } from "./button.tsx";
import { IconButton } from "./icon-button.tsx";
import { Icon } from "./icons/icon.tsx";
import {
  dismissToast,
  removeToast,
  subscribeToasts,
  type ToastRecord,
  toastSnapshot,
} from "./toast-store.ts";
import "./toast.css";

const VISIBLE = 3;

const GAP = 8;

const PEEK = 10;

const TONE_ICON = { success: "check", danger: "alert" } as const;

/**
 * Where toasts appear: bottom-left, newest in front. Collapsed, older toasts peek out behind it as
 * a stack; hovering or focusing the stack fans it out, and pauses every dwell timer.
 */
export function Toaster() {
  const records = useSyncExternalStore(subscribeToasts, toastSnapshot, toastSnapshot);
  const regionRef = useRef<HTMLElement | null>(null);
  const [expanded, setExpanded] = useState(false);
  const [heights, setHeights] = useState<ReadonlyMap<number, number>>(new Map());

  // Re-show the region on each new toast so it sits above any dialog opened since.
  useLayoutEffect(() => {
    const region = regionRef.current;

    if (region === null || !supportsPopover(region) || records.length === 0) {
      return;
    }

    hidePopover(region);
    showPopover(region);
  }, [records.length]);

  const newestFirst = [...records].reverse();
  const live = newestFirst.filter((record) => !record.closing);

  const onHeight = (id: number, height: number) => {
    setHeights((current) => {
      if (current.get(id) === height) {
        return current;
      }

      const next = new Map(current);

      next.set(id, height);

      return next;
    });
  };

  return (
    <section
      ref={regionRef}
      className="toaster"
      aria-label="Notifications"
      popover="manual"
      data-expanded={expanded || undefined}
      onPointerEnter={() => setExpanded(true)}
      onPointerLeave={() => setExpanded(false)}
      onFocus={() => setExpanded(true)}
      onBlur={(event) => {
        if (
          !(
            event.relatedTarget instanceof Node && event.currentTarget.contains(event.relatedTarget)
          )
        ) {
          setExpanded(false);
        }
      }}
    >
      <ol className="toaster-list">
        {newestFirst.map((record) => {
          const index = Math.max(0, live.indexOf(record));
          const above = live.slice(0, index);

          const offset = expanded
            ? above.reduce((sum, item) => sum + (heights.get(item.id) ?? 56) + GAP, 0)
            : index * PEEK;

          return (
            <ToastItem
              key={record.id}
              record={record}
              index={index}
              offset={offset}
              expanded={expanded}
              paused={expanded}
              onHeight={onHeight}
            />
          );
        })}
      </ol>
    </section>
  );
}

interface ToastItemProps {
  readonly record: ToastRecord;
  readonly index: number;
  readonly offset: number;
  readonly expanded: boolean;
  readonly paused: boolean;
  readonly onHeight: (id: number, height: number) => void;
}

function ToastItem({ record, index, offset, expanded, paused, onHeight }: ToastItemProps) {
  const ref = useRef<HTMLDivElement | null>(null);
  const remaining = useRef(-1);
  const { id, closing } = record;

  useLayoutEffect(() => {
    const element = ref.current;

    if (element === null) {
      return;
    }

    onHeight(id, element.offsetHeight);

    if (!("ResizeObserver" in window)) {
      return;
    }

    const observer = new ResizeObserver(() => onHeight(id, element.offsetHeight));

    observer.observe(element);

    return () => observer.disconnect();
  }, [id, onHeight]);

  // Dwell, then leave on its own; paused while the stack is hovered or focused.
  useEffect(() => {
    if (paused || closing) {
      return;
    }

    if (remaining.current < 0) {
      remaining.current = readDurationMs("--duration-toast-dwell") || 5000;
    }

    const started = performance.now();
    const timer = window.setTimeout(() => dismissToast(id), remaining.current);

    return () => {
      window.clearTimeout(timer);
      remaining.current -= performance.now() - started;
    };
  }, [paused, closing, id]);

  useEffect(() => {
    const element = ref.current;

    if (!closing || element === null) {
      return;
    }

    return afterExit(element, () => removeToast(id));
  }, [closing, id]);

  const hidden = index >= VISIBLE && !closing;
  const scale = expanded || closing ? 1 : 1 - index * 0.04;

  return (
    <li
      className="toast-slot"
      data-front={index === 0 || undefined}
      data-hidden={hidden || undefined}
      style={{ translate: `0 ${-offset}px`, scale: `${scale}`, zIndex: 100 - index }}
    >
      <div
        ref={ref}
        className="toast t-toast"
        data-state={closing ? "closing" : "open"}
        data-tone={record.tone ?? "neutral"}
        role={record.tone === "danger" ? "alert" : "status"}
        inert={hidden}
      >
        <div className="toast-body">
          {record.tone === "success" || record.tone === "danger" ? (
            <span className="toast-icon" aria-hidden="true">
              <Icon name={TONE_ICON[record.tone]} />
            </span>
          ) : null}
          <div className="toast-text">
            <p className="toast-title">{record.title}</p>
            {record.description === undefined ? null : (
              <p className="toast-description">{record.description}</p>
            )}
          </div>
          {record.action === undefined ? null : (
            <Button
              size="sm"
              variant="secondary"
              onClick={() => {
                record.action?.onClick();
                dismissToast(id);
              }}
            >
              {record.action.label}
            </Button>
          )}
          <IconButton icon="x" label="Dismiss" size="sm" onClick={() => dismissToast(id)} />
        </div>
      </div>
    </li>
  );
}
