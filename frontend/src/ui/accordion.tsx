import { type ReactNode, useId, useState } from "react";
import { Icon } from "./icons/icon.tsx";
import "./accordion.css";

interface AccordionProps {
  readonly title: ReactNode;
  readonly defaultOpen?: boolean;
  readonly children: ReactNode;
}

/**
 * A disclosure section (sidebar groups, settings). The panel's height animates from 0fr to 1fr
 * with no measuring (the transitions.dev accordion); a closed panel is inert.
 */
export function Accordion({ title, defaultOpen = false, children }: AccordionProps) {
  const id = useId();
  const [open, setOpen] = useState(defaultOpen);

  return (
    <div className="accordion t-acc" data-open={open}>
      <h3 className="accordion-heading">
        <button
          type="button"
          id={`${id}-trigger`}
          className="accordion-trigger"
          aria-expanded={open}
          aria-controls={`${id}-panel`}
          onClick={() => setOpen(!open)}
        >
          <span className="t-acc-chevron">
            <Icon name="chevron-down" size={14} />
          </span>
          <span>{title}</span>
        </button>
      </h3>
      <section
        id={`${id}-panel`}
        className="t-acc-panel"
        aria-labelledby={`${id}-trigger`}
        inert={!open}
      >
        <div className="t-acc-panel-inner">
          <div className="accordion-content">{children}</div>
        </div>
      </section>
    </div>
  );
}
