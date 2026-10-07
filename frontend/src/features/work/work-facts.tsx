/**
 * Work facts wherever a thread shows: the status pill, the owner (or "Unassigned", and "owner
 * inactive" when they can't act on it), the linked pull requests, events and Drive files, and the
 * agent's run.
 */
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import { TextSwap } from "../../motion/text-swap.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { UNKNOWN_NAME, useUser } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import {
  linkAccessibleName,
  linkDetail,
  linkIcon,
  UNASSIGNED,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
} from "./work-format.ts";
import "./work.css";

const STATUS_LABELS = WORK_STATUSES.map((status) => WORK_STATUS_LABEL[status]);

/** A status pill: a dot that says the state at a glance, and the label. */
export function WorkStatusPill({ status }: { readonly status: WorkStatus }) {
  return (
    <span className="work-status" data-status={status}>
      <span className="work-status-dot" aria-hidden="true" />
      <TextSwap reserve={STATUS_LABELS}>{WORK_STATUS_LABEL[status]}</TextSwap>
    </span>
  );
}

/** The owner's face and name, "Unassigned", or the name with "owner inactive". */
export function WorkOwner({
  ownerId,
  active,
  size = 16,
}: {
  readonly ownerId: number | null;
  readonly active: boolean;
  readonly size?: number;
}) {
  const owner = useUser(ownerId ?? undefined);

  if (ownerId === null) {
    return (
      <span className="work-owner" data-unassigned="">
        <Icon name="user-plus" size={size - 2} />
        {UNASSIGNED}
      </span>
    );
  }

  return (
    <span className="work-owner" data-inactive={active ? undefined : ""}>
      <UserAvatar userId={ownerId} size={size} decorative />
      <span className="work-owner-name">{owner?.name ?? UNKNOWN_NAME}</span>
      {active ? null : <span className="work-owner-inactive">owner inactive</span>}
    </span>
  );
}

/** One linked item: its kind icon, label, and the PR state or event time. Opens in a new tab. */
export function WorkLinkChip({ link }: { readonly link: WorkLink }) {
  const detail = linkDetail(link);
  const external = /^https?:\/\//i.test(link.url);

  return (
    <a
      className="work-link"
      href={link.url}
      data-kind={link.kind}
      data-state={link.pullRequestState ?? undefined}
      data-cancelled={link.eventCancelled || undefined}
      aria-label={linkAccessibleName(link)}
      title={link.title ?? link.label}
      {...(external ? { target: "_blank", rel: "noopener noreferrer" } : {})}
    >
      <Icon name={linkIcon(link.kind)} size={12} />
      <span className="work-link-label">{link.label}</span>
      {detail === null ? null : <span className="work-link-detail">{detail}</span>}
    </a>
  );
}

/** The agent's run (a CI job, a session), opened in a new tab. */
export function RunLink({ url }: { readonly url: string }) {
  return (
    <a
      className="work-link"
      data-kind="run"
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      aria-label="Run, opens in a new tab"
    >
      <Icon name="rocket" size={12} />
      <span className="work-link-label">Run</span>
      <Icon name="arrow-up-right" size={12} className="work-link-out" />
    </a>
  );
}

/** The links and the run, in a wrapping row; nothing when there are none. */
export function WorkLinks({
  links,
  runUrl,
  label,
}: {
  readonly links: readonly WorkLink[];
  readonly runUrl: string | null;
  /** The list's accessible name, e.g. "Links for Pricing page copy". */
  readonly label: string;
}) {
  if (links.length === 0 && runUrl === null) {
    return null;
  }

  return (
    <ul className="work-links" aria-label={label}>
      {runUrl === null ? null : (
        <li>
          <RunLink url={runUrl} />
        </li>
      )}
      {links.map((link) => (
        <li key={link.id}>
          <WorkLinkChip link={link} />
        </li>
      ))}
    </ul>
  );
}

/**
 * The status and owner on one line, for a thread row or the indicator under a message. Plain
 * text inside the row's button, so it reads with the row; links go beside it (see `WorkLinks`).
 */
export function WorkSummary({ facts }: { readonly facts: WorkFacts }) {
  return (
    <span className="work-summary">
      <span className="visually-hidden">Work: </span>
      <WorkStatusPill status={facts.status} />
      <WorkOwner ownerId={facts.ownerId} active={facts.ownerActive} />
    </span>
  );
}
