import { Link, useMatchRoute } from "@tanstack/react-router";
import { createContext, type ReactNode, use } from "react";
import { Icon, type IconName } from "../../ui/icons/icon.tsx";
import { PageHeader } from "../../ui/page-header.tsx";
import { usePhoneLayout } from "../panes/use-right-pane.ts";
import { groupSections, type Section, type SectionPath } from "./settings-format.ts";

/** The section a page is in, and whether it is that section's own page or one under it. */
interface OpenSection<S extends Section> {
  readonly section: S;
  readonly page: SectionPath;
  readonly nested: boolean;
}

/**
 * Where the router is among `sections`: null on the root itself (on phones, the list), else the
 * section whose page, or `under` path, the location is or is below. Sections don't nest, so the
 * first that matches is the one.
 */
function useOpenSection<S extends Section>(
  root: SectionPath,
  sections: readonly S[],
): OpenSection<S> | null {
  const matchRoute = useMatchRoute();

  if (matchRoute({ to: root }) !== false) {
    return null;
  }

  const inside = (prefix: SectionPath | undefined) =>
    prefix !== undefined && matchRoute({ to: prefix, fuzzy: true }) !== false;

  for (const section of sections) {
    const page = section.page ?? section.path;

    if (inside(page) || inside(section.under)) {
      return { section, page, nested: matchRoute({ to: page }) === false };
    }
  }

  return null;
}

/**
 * The pushed page's title, on phones, where the header names the section: a page whose own
 * heading says the same hides it there rather than say it twice.
 */
const PushedTitleContext = createContext<string | null>(null);

/** Whether the header already shows `title` (a phone's pushed section page). */
export function useTitleInHeader(title: string): boolean {
  return use(PushedTitleContext) === title;
}

/**
 * The settings and workspace pages' frame: the header, the section nav and the open section.
 * Wide screens show the nav beside the section. On phones they are list and detail, as iOS and
 * Slack lay out settings: the root is a grouped list of the sections, and a section is a page
 * pushed over it, its header's back button returning to the list (or, under a section, to it).
 */
export function SectionsLayout<S extends Section>({
  root,
  title,
  icon,
  navLabel,
  sections,
  children,
}: {
  readonly root: SectionPath;
  readonly title: string;
  readonly icon: IconName;
  /** The nav's accessible name ("Settings sections"). */
  readonly navLabel: string;
  readonly sections: readonly S[];
  /** The open section's content (or the page's loading and error states). */
  readonly children: ReactNode;
}) {
  const phone = usePhoneLayout();
  const atRoot = useMatchRoute()({ to: root }) !== false;
  const open = useOpenSection(root, sections);
  const pushed = phone ? open : null;
  // The router marks the link to where it is; a root section's own page (`page`) is marked here.
  const onPage = open?.nested === false ? open.section : null;

  return (
    <div className="settings" data-at={atRoot ? "list" : undefined}>
      {pushed === null ? (
        <PageHeader
          className="settings-header"
          back={{
            label: "Back to conversations",
            link: (props) => <Link to="/" {...props} />,
          }}
          title={
            <>
              <Icon name={icon} size={18} className="settings-header-icon" />
              <span className="settings-header-title text-title">{title}</span>
            </>
          }
        />
      ) : (
        <PageHeader
          className="settings-header"
          back={{
            label: `Back to ${pushed.nested ? pushed.section.label : title}`,
            link: (props) => <Link to={pushed.nested ? pushed.page : root} {...props} />,
          }}
          title={<span className="settings-header-title text-title">{pushed.section.label}</span>}
        />
      )}
      <div className="settings-body">
        <nav className="settings-nav" aria-label={navLabel}>
          {groupSections(sections).map((group) => (
            <ul key={group[0]?.key}>
              {group.map((section) => (
                <li key={section.key}>
                  <Link
                    to={phone ? (section.page ?? section.path) : section.path}
                    className="settings-nav-link"
                    activeOptions={{ exact: true }}
                    activeProps={{ "aria-current": "page" }}
                    aria-current={section === onPage ? "page" : undefined}
                  >
                    <Icon name={section.icon} size={16} />
                    <span className="settings-nav-label">{section.label}</span>
                    <Icon name="chevron-right" size={16} className="settings-nav-chevron" />
                  </Link>
                </li>
              ))}
            </ul>
          ))}
        </nav>
        <div className="settings-content">
          <PushedTitleContext value={pushed?.section.label ?? null}>{children}</PushedTitleContext>
        </div>
      </div>
    </div>
  );
}

/**
 * The root's own route (`/settings`, `/admin`): the first section beside the nav, and nothing on
 * phones, where the root is the list.
 */
export function RootSection({ children }: { readonly children: ReactNode }) {
  return usePhoneLayout() ? null : children;
}
