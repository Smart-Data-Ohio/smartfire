import {
  type KeyboardEvent,
  type ReactNode,
  useId,
  useLayoutEffect,
  useRef,
  useState,
} from "react";
import { Icon, type IconName } from "./icons/icon.tsx";
import "./tabs.css";

export interface TabItem {
  readonly value: string;
  readonly label: string;
  readonly icon?: IconName;
}

interface TabsProps {
  readonly items: readonly TabItem[];
  readonly value: string;
  readonly onValueChange: (value: string) => void;
  /** The tab list's accessible name. */
  readonly label: string;
  /** The selected tab's panel. */
  readonly children?: ReactNode;
}

/**
 * Tabs with a pill that slides to the selected tab (the transitions.dev "tabs sliding" recipe).
 * WAI-ARIA tabs with automatic activation: arrows move and select, Home/End jump, and only the
 * selected tab is in the Tab order.
 */
export function Tabs({ items, value, onValueChange, label, children }: TabsProps) {
  const id = useId();
  const listRef = useRef<HTMLDivElement | null>(null);
  const pillRef = useRef<HTMLSpanElement | null>(null);
  const [measured, setMeasured] = useState(false);

  // biome-ignore lint/correctness/useExhaustiveDependencies: re-measure when the selection or the tabs change
  useLayoutEffect(() => {
    const list = listRef.current;
    const pill = pillRef.current;

    if (list === null || pill === null) {
      return;
    }

    const measure = () => {
      const tab = list.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');

      if (tab === null) {
        return;
      }

      pill.style.width = `${tab.offsetWidth}px`;
      pill.style.transform = `translateX(${tab.offsetLeft}px)`;
    };

    measure();

    // The first position lands without a slide; later changes animate.
    const frame = requestAnimationFrame(() => setMeasured(true));

    if (!("ResizeObserver" in window)) {
      return () => cancelAnimationFrame(frame);
    }

    const observer = new ResizeObserver(measure);

    observer.observe(list);

    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [value, items]);

  const select = (index: number) => {
    const item = items[(index + items.length) % items.length];

    if (item === undefined) {
      return;
    }

    onValueChange(item.value);
    const tabs = listRef.current?.querySelectorAll<HTMLElement>('[role="tab"]') ?? [];

    [...tabs].find((tab) => tab.dataset.value === item.value)?.focus();
  };

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const index = items.findIndex((item) => item.value === value);

    const moves = new Map([
      ["ArrowRight", index + 1],
      ["ArrowLeft", index - 1],
      ["Home", 0],
      ["End", items.length - 1],
    ]);

    const next = moves.get(event.key);

    if (next !== undefined) {
      event.preventDefault();
      select(next);
    }
  };

  return (
    <div className="tabs">
      <div ref={listRef} role="tablist" aria-label={label} className="t-tabs" onKeyDown={onKeyDown}>
        {items.map((item) => {
          const selected = item.value === value;

          return (
            <button
              key={item.value}
              type="button"
              role="tab"
              id={`${id}-tab-${item.value}`}
              className="tab t-tab"
              data-value={item.value}
              aria-selected={selected}
              aria-controls={selected ? `${id}-panel` : undefined}
              tabIndex={selected ? 0 : -1}
              onClick={() => onValueChange(item.value)}
            >
              {item.icon === undefined ? null : <Icon name={item.icon} size={14} />}
              {item.label}
            </button>
          );
        })}
        <span ref={pillRef} className="t-tabs-pill" data-measured={measured} aria-hidden="true" />
      </div>
      {children === undefined ? null : (
        <div
          role="tabpanel"
          id={`${id}-panel`}
          aria-labelledby={`${id}-tab-${value}`}
          className="tab-panel"
          // biome-ignore lint/a11y/noNoninteractiveTabindex: APG tab panels take focus when they hold no focusable content
          tabIndex={0}
        >
          {children}
        </div>
      )}
    </div>
  );
}
