import type { ComponentPropsWithRef, ReactElement, ReactNode } from "react";
import { Badge } from "./badge.tsx";
import { IconButton } from "./icon-button.tsx";
import { Icon, type IconName } from "./icons/icon.tsx";
import { Menu } from "./menu.tsx";
import "./page-header.css";

/** What a back link's element gets: spread onto the router `Link` that `PageHeaderBack.link` renders. */
export interface PageBackProps {
  readonly className: string;
  readonly "aria-label": string;
  readonly children: ReactNode;
}

/** The way back: a link (a router `Link`, rendered by `link`) or a button (`onBack`). */
export type PageHeaderBack = {
  readonly label: string;
  /**
   * Shown on every screen (a pane over a thread). By default it shows on phones only, where the
   * page is pushed full screen over the one it came from.
   */
  readonly always?: boolean;
  /** The glyph; a chevron by default. */
  readonly icon?: IconName;
} & (
  | { readonly link: (props: PageBackProps) => ReactElement; readonly onBack?: never }
  | { readonly onBack: () => void; readonly link?: never }
);

export interface PageHeaderProps extends Omit<ComponentPropsWithRef<"header">, "title"> {
  readonly back?: PageHeaderBack | undefined;
  /** The heading: the caller's element (an `h1`, or a button holding one), with any glyph. */
  readonly title: ReactNode;
  /** A muted line under the title (a room, a member count); the title and it then stack. */
  readonly subtitle?: ReactNode;
  /** The subtitle's id, for a heading's `aria-describedby`. */
  readonly subtitleId?: string;
  /** Trailing buttons, always shown: two at most on phones, so the title keeps its width. */
  readonly actions?: ReactNode;
  /** The ⋯ menu's items (`MenuItem`s): the tools that don't fit beside the title. */
  readonly overflow?: ReactNode | undefined;
  readonly overflowLabel?: string;
  /**
   * Something in the ⋯ menu that wants attention (raised hands on a stage): a dot on the button,
   * and its label (`"2 raised hands"`) added to the button's name. Hidden without one.
   */
  readonly overflowAlert?: string | undefined;
  /** Opens the ⋯ menu from outside (a URL that names one of its items); uncontrolled without. */
  readonly overflowMenu?:
    | { readonly open: boolean; readonly onOpenChange: (open: boolean) => void }
    | undefined;
}

function BackAffordance({ back }: { readonly back: PageHeaderBack }) {
  const icon = back.icon ?? "chevron-left";

  if (back.onBack !== undefined) {
    return (
      <IconButton
        icon={icon}
        label={back.label}
        className="page-header-back"
        tooltipPlacement="bottom"
        onClick={back.onBack}
      />
    );
  }

  return back.link({
    className: "page-header-back",
    "aria-label": back.label,
    children: <Icon name={icon} size={20} />,
  });
}

/**
 * A screen's title bar, the room's and every page's and pane's: back, the title (and a subtitle
 * under it), up to two actions and a ⋯ menu for the rest. 48 px on every screen; on phones the
 * back button and the actions take the 44 px touch size, and the title keeps whatever width is
 * left. The shell keeps it clear of the status bar (--safe-top).
 */
export function PageHeader({
  back,
  title,
  subtitle,
  subtitleId,
  actions,
  overflow,
  overflowLabel = "More",
  overflowAlert,
  overflowMenu,
  className,
  ...rest
}: PageHeaderProps) {
  const backShows = back === undefined ? undefined : back.always === true ? "always" : "phone";

  return (
    <header
      {...rest}
      className={className === undefined ? "page-header" : `page-header ${className}`}
      data-back={backShows}
    >
      {back === undefined ? null : <BackAffordance back={back} />}
      <div className="page-header-heading" data-stacked={subtitle === undefined ? undefined : true}>
        {title}
        {subtitle === undefined ? null : (
          <div id={subtitleId} className="page-header-subtitle">
            {subtitle}
          </div>
        )}
      </div>
      {actions === undefined && overflow === undefined ? null : (
        <div className="page-header-actions">
          {actions}
          {overflow === undefined ? null : (
            <Menu
              {...overflowMenu}
              // A menu under another name is another menu: it opens fresh, focus on its first item.
              key={overflowLabel}
              placement="bottom-end"
              label={overflowLabel}
              trigger={(props) => (
                <span className="page-header-overflow-wrap">
                  <IconButton
                    {...props}
                    icon="more"
                    label={
                      overflowAlert === undefined
                        ? overflowLabel
                        : `${overflowLabel} (${overflowAlert})`
                    }
                    tooltipPlacement="bottom-end"
                    className="page-header-overflow"
                  />
                  <Badge count={overflowAlert === undefined ? 0 : 1} dot floating label="" />
                </span>
              )}
            >
              {overflow}
            </Menu>
          )}
        </div>
      )}
    </header>
  );
}
