/**
 * Work facts wherever a thread shows: the status pill, the owner (or "Unassigned", and "owner
 * inactive" when they can't act on it), the linked pull requests, events and Drive files, and the
 * agent's run.
 */
import type { User } from "../../gen/User.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkLink } from "../../gen/WorkLink.ts";
import { TextSwap } from "../../motion/text-swap.tsx";
import { isSafeWorkHref } from "../../sync/runtime.ts";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { Avatar } from "../../ui/avatar.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { isAgent, useUser } from "../people/people.ts";
import {
  isKnownStatus,
  linkAccessibleName,
  linkDetail,
  linkIcon,
  UNASSIGNED,
  WORK_STATUS_LABEL,
  WORK_STATUSES,
  workStatusLabel,
} from "./work-format.ts";
import "./work.css";

// The four real labels hold the pill's width; the rare unknown one simply widens it.
const STATUS_LABELS = WORK_STATUSES.map((status) => WORK_STATUS_LABEL[status]);

/**
 * A status pill: a dot that says the state at a glance, and the label. A status this client
 * doesn't know (read tolerantly as `"unknown"`) gets a neutral pill.
 */
export function WorkStatusPill({ status }: { readonly status: string }) {
  return (
    <span className="work-status" data-status={isKnownStatus(status) ? status : "unknown"}>
      <span className="work-status-dot" aria-hidden="true" />
      <TextSwap reserve={STATUS_LABELS}>{workStatusLabel(status)}</TextSwap>
    </span>
  );
}

/**
 * The owner's avatar, from the whole user the facts carry (thread events bring no `users`); the
 * store's copy wins when it has one, since it follows renames.
 */
export function OwnerAvatar({
  owner,
  size = 16,
}: {
  readonly owner: User;
  readonly size?: number;
}) {
  const user = useUser(owner.id) ?? owner;

  if (isAgent(user)) {
    return <AgentAvatar seed={`${user.id}`} name={user.name} size={size} decorative />;
  }

  return <Avatar name={user.name} userId={user.id} src={user.avatarUrl} size={size} decorative />;
}

/** The owner's face and name, "Unassigned", or the name with "owner inactive". */
export function WorkOwner({
  owner,
  active,
  size = 16,
}: {
  readonly owner: User | null;
  readonly active: boolean;
  readonly size?: number;
}) {
  const known = useUser(owner?.id);

  if (owner === null) {
    return (
      <span className="work-owner" data-unassigned="">
        <Icon name="user-plus" size={size - 2} />
        {UNASSIGNED}
      </span>
    );
  }

  return (
    <span className="work-owner" data-inactive={active ? undefined : ""}>
      <OwnerAvatar owner={owner} size={size} />
      <span className="work-owner-name">{known?.name ?? owner.name}</span>
      {active ? null : <span className="work-owner-inactive">owner inactive</span>}
    </span>
  );
}

/**
 * One linked item: its kind icon, label, and the PR state or event time. A pull request or Drive
 * file opens in a new tab; an event opens its classic page. A URL that isn't safe in an `href`
 * (the decoder drops those already) shows as plain text.
 */
export function WorkLinkChip({ link }: { readonly link: WorkLink }) {
  const detail = linkDetail(link);
  const external = /^https:\/\//i.test(link.url);

  if (!isSafeWorkHref(link.url)) {
    return (
      <span className="work-link" data-kind={link.kind}>
        <Icon name={linkIcon(link.kind)} size={12} />
        <span className="work-link-label">{link.label}</span>
      </span>
    );
  }

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

/** The agent's run (a CI job, a session), opened in a new tab; nothing for an unsafe URL. */
export function RunLink({ url }: { readonly url: string }) {
  if (!isSafeWorkHref(url)) {
    return null;
  }

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
      <WorkOwner owner={facts.owner} active={facts.ownerActive} />
    </span>
  );
}
