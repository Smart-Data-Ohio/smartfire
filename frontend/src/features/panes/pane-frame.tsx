import { createContext, type ReactNode, use, useId } from "react";
import { shortcutKeys } from "../../lib/shortcuts.ts";
import { useStore } from "../../store/store.ts";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { PageHeader } from "../../ui/page-header.tsx";

/** What the right pane's frame needs from the container: how to go back and how to close. */
export interface PaneChrome {
  /** Uncovers what's under this pane (a side pane under a thread, or the conversation on phones). */
  readonly onBack: (() => void) | null;
  readonly backLabel: string;
  readonly onClose: () => void;
  readonly phone: boolean;
  /** The heading's id, for the pane's accessible name. */
  readonly headingId: string;
}

export const PaneChromeContext = createContext<PaneChrome>({
  onBack: null,
  backLabel: "Back",
  onClose: () => {},
  phone: false,
  headingId: "right-pane-heading",
});

interface PaneFrameProps {
  readonly title: ReactNode;
  /** A muted line under the title: the room, a status. */
  readonly subtitle?: ReactNode;
  /** Header buttons before the close button. */
  readonly tools?: ReactNode;
  /** Sits under the header and doesn't scroll (tabs, a search field). */
  readonly toolbar?: ReactNode;
  /** Sits at the bottom and doesn't scroll (a composer). */
  readonly footer?: ReactNode;
  readonly children: ReactNode;
}

/**
 * The right pane's frame: a page header that lines up with the conversation's (back, title over
 * subtitle, tools, close), an optional fixed toolbar, the scrolling body and an optional fixed
 * footer. On phones the back chevron leads and the close button goes, as a pushed page has it.
 */
export function PaneFrame({ title, subtitle, tools, toolbar, footer, children }: PaneFrameProps) {
  const chrome = use(PaneChromeContext);
  const subtitleId = useId();

  return (
    <div className="pane-frame">
      <PageHeader
        className="pane-header"
        back={
          chrome.onBack === null
            ? undefined
            : {
                label: chrome.backLabel,
                icon: chrome.phone ? "chevron-left" : "arrow-left",
                always: true,
                onBack: chrome.onBack,
              }
        }
        title={
          <h2
            id={chrome.headingId}
            className="pane-title"
            aria-describedby={subtitle === undefined ? undefined : subtitleId}
          >
            {title}
          </h2>
        }
        subtitle={subtitle}
        subtitleId={subtitleId}
        actions={
          <>
            {tools}
            {chrome.phone ? null : (
              <IconButton
                icon="x"
                label="Close"
                shortcut={shortcutKeys("close-pane")}
                tooltipPlacement="bottom-end"
                onClick={chrome.onClose}
              />
            )}
          </>
        }
      />
      {toolbar === undefined ? null : <div className="pane-toolbar">{toolbar}</div>}
      <div className="pane-body">{children}</div>
      {footer === undefined ? null : <div className="pane-footer">{footer}</div>}
    </div>
  );
}

/** "# general" (or the person, in a DM): a pane's subtitle naming the conversation it's about. */
export function RoomName({ roomId }: { readonly roomId: number }) {
  const name = useStore(
    (state) =>
      state.rooms[roomId]?.detail?.displayName ?? state.sidebar.rows[roomId]?.displayName ?? "",
  );

  const direct = useStore(
    (state) =>
      (state.rooms[roomId]?.detail?.room.kind ?? state.sidebar.rows[roomId]?.room.kind) ===
      "direct",
  );

  return (
    <span className="pane-room-name">
      {direct ? null : <Icon name="hash" size={12} />}
      {name}
    </span>
  );
}
