import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import "./destinations.css";

interface PageFrameProps {
  readonly title: string;
  readonly icon: IconName;
  /** Beside the title: a count, a status. */
  readonly meta?: ReactNode;
  /** Header controls at the end (a filter toggle). */
  readonly tools?: ReactNode;
  /** Sits under the header and doesn't scroll (tabs). */
  readonly toolbar?: ReactNode;
  /**
   * On phones, a back chevron to the conversation list: for a page pushed from the list (Saved,
   * Scheduled), not for one the tab bar selects (Activity).
   */
  readonly back?: boolean;
  readonly children: ReactNode;
}

/**
 * A workspace destination's page (Activity, Saved, Scheduled) in the main pane: a 48 px header
 * that lines up with a conversation's (glyph, title, meta, tools), an optional fixed toolbar and
 * the body. On phones it is a full screen with the room header's back affordance.
 */
export function PageFrame({
  title,
  icon,
  meta,
  tools,
  toolbar,
  back = false,
  children,
}: PageFrameProps) {
  return (
    <section className="page" aria-labelledby="page-title">
      <header className="page-header">
        {back ? (
          <Link to="/" className="page-back" aria-label="Back to conversations">
            <Icon name="chevron-left" size={20} />
          </Link>
        ) : null}
        <div className="page-heading">
          <Icon name={icon} size={18} className="page-heading-icon" />
          <h1 id="page-title" className="page-title">
            {title}
          </h1>
          {meta}
        </div>
        {tools === undefined ? null : <div className="page-tools">{tools}</div>}
      </header>
      {toolbar === undefined ? null : <div className="page-toolbar">{toolbar}</div>}
      <div className="page-body">{children}</div>
    </section>
  );
}
