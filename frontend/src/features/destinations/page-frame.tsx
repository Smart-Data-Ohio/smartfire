import { Link } from "@tanstack/react-router";
import type { ReactNode } from "react";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { PageHeader, type PageHeaderBack } from "../../ui/page-header.tsx";
import "./destinations.css";

interface PageFrameProps {
  readonly title: string;
  readonly icon: IconName;
  /** Beside the title: a count, a status. */
  readonly meta?: ReactNode;
  /** Header controls at the end (a filter toggle). */
  readonly tools?: ReactNode;
  /** The header's ⋯ menu items: on phones, the controls that don't fit beside the title. */
  readonly overflow?: ReactNode;
  /** Sits under the header and doesn't scroll (tabs). */
  readonly toolbar?: ReactNode;
  /**
   * On phones, a back chevron: `true` to the conversation list, for a page pushed from the list
   * (Saved, Scheduled), not for one the tab bar selects (Activity); or somewhere else (an event
   * back to its room's calendar).
   */
  readonly back?: boolean | PageHeaderBack;
  readonly children: ReactNode;
}

const TO_CONVERSATIONS: PageHeaderBack = {
  label: "Back to conversations",
  link: (props) => <Link to="/" {...props} />,
};

function backOf(back: boolean | PageHeaderBack): PageHeaderBack | undefined {
  if (back === true) {
    return TO_CONVERSATIONS;
  }

  return back === false ? undefined : back;
}

/**
 * A workspace destination's page (Activity, Saved, Scheduled) in the main pane: a page header
 * that lines up with a conversation's (glyph, title, meta, tools), an optional fixed toolbar and
 * the body. On phones it is a full screen with the room header's back affordance.
 */
export function PageFrame({
  title,
  icon,
  meta,
  tools,
  overflow,
  toolbar,
  back = false,
  children,
}: PageFrameProps) {
  return (
    <section className="page" aria-labelledby="page-title">
      <PageHeader
        back={backOf(back)}
        title={
          <>
            <Icon name={icon} size={18} className="page-heading-icon" />
            <h1 id="page-title" className="page-title">
              {title}
            </h1>
            {meta}
          </>
        }
        actions={tools}
        overflow={overflow}
      />
      {toolbar === undefined ? null : <div className="page-toolbar">{toolbar}</div>}
      <div className="page-body">{children}</div>
    </section>
  );
}
