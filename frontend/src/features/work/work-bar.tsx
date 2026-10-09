/**
 * The thread pane's work section (S4): under the header of a tracked thread, the status and the
 * owner (menus for whoever may change them), the links and the run, and a disclosure with the
 * result, the agent's steps, the history and "Hand off to an agent". "Link" opens the editor at
 * `/links`, which is also where a classic links URL lands, including for a thread that isn't a
 * board post. `WorkLive` sits beside it in the pane, announcing changes and refetching the
 * detail when live facts move on.
 */
import { useEffect, useRef, useState } from "react";
import type { User } from "../../gen/User.ts";
import type { WorkDetail } from "../../gen/WorkDetail.ts";
import type { WorkFacts } from "../../gen/WorkFacts.ts";
import type { WorkOwnerCandidate } from "../../gen/WorkOwnerCandidate.ts";
import type { WorkStatus } from "../../gen/WorkStatus.ts";
import type { ThreadPermissions } from "../../store/model.ts";
import { useStore } from "../../store/store.ts";
import { workDetailStale } from "../../store/work.ts";
import { actions } from "../../sync/runtime.ts";
import { Accordion } from "../../ui/accordion.tsx";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { Menu, MenuGroup, MenuItem, MenuRadioItem, MenuSeparator } from "../../ui/menu.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PostLinks } from "../boards/post-links.tsx";
import { useAnnouncer } from "../destinations/live-region.tsx";
import { isAgent, UNKNOWN_NAME } from "../people/people.ts";
import { UserAvatar } from "../people/user-avatar.tsx";
import { threadTitle } from "../threads/thread-format.ts";
import { handoffRefusal } from "./handoff-access.ts";
import { HandoffDialog, useHandoffRoute } from "./handoff-dialog.tsx";
import { useLinksRoute } from "./links-route.ts";
import { WorkHistory, WorkResult, WorkStepsSection } from "./work-details.tsx";
import { WorkLinks, WorkOwner, WorkStatusPill } from "./work-facts.tsx";
import { UNASSIGNED, WORK_STATUS_LABEL, WORK_STATUSES, workStatusLabel } from "./work-format.ts";
import "./work.css";

function failed(title: string) {
  return (error: Error) => toast({ title, description: error.message, tone: "danger" });
}

/** The status, as a menu of the four statuses (and "Stop tracking…") when the viewer may set it. */
function StatusControl({
  threadId,
  facts,
  permissions,
  onStop,
}: {
  readonly threadId: number;
  readonly facts: WorkFacts;
  readonly permissions: ThreadPermissions;
  readonly onStop: () => void;
}) {
  if (!permissions.canUpdateWorkStatus) {
    return <WorkStatusPill status={facts.status} />;
  }

  const choose = (status: WorkStatus) => {
    if (status !== facts.status) {
      actions.work.setStatus(threadId, status).catch(failed("Couldn't change the status"));
    }
  };

  return (
    <Menu
      label="Work status"
      trigger={(props) => (
        <Button
          {...props}
          variant="ghost"
          size="sm"
          trailingIcon="chevron-down"
          className="work-control"
          aria-label={`Status: ${workStatusLabel(facts.status)}. Change status`}
        >
          <WorkStatusPill status={facts.status} />
        </Button>
      )}
    >
      {WORK_STATUSES.map((status) => (
        <MenuRadioItem
          key={status}
          checked={facts.status === status}
          onSelect={() => choose(status)}
        >
          {WORK_STATUS_LABEL[status]}
        </MenuRadioItem>
      ))}
      {permissions.canRemoveWork ? (
        <>
          <MenuSeparator />
          <MenuItem icon="x" tone="danger" onSelect={onStop}>
            Stop tracking…
          </MenuItem>
        </>
      ) : null}
    </Menu>
  );
}

/** "Anthropic · Workspace agent: triage, drafts and PR reviews", when the server gave either. */
function candidateDetail(candidate: WorkOwnerCandidate): string | null {
  const detail = [candidate.provider, candidate.description]
    .filter((part) => part !== null && part !== "")
    .join(" · ");

  return detail === "" ? null : detail;
}

/** One owner to choose in the menu. */
function OwnerOption({
  candidate,
  user,
  checked,
  onSelect,
}: {
  readonly candidate: WorkOwnerCandidate;
  readonly user: User | undefined;
  readonly checked: boolean;
  readonly onSelect: () => void;
}) {
  const name = user?.name ?? UNKNOWN_NAME;
  const detail = candidateDetail(candidate);

  return (
    <MenuRadioItem checked={checked} onSelect={onSelect} label={name}>
      <span className="work-owner-option">
        <UserAvatar userId={candidate.userId} size={20} decorative />
        <span className="work-owner-option-text">
          <span>{name}</span>
          {detail === null ? null : <span className="work-owner-option-detail">{detail}</span>}
        </span>
      </span>
    </MenuRadioItem>
  );
}

/** The owner, as a menu of `ownerCandidates` (people, then agents) when the viewer may assign. */
function OwnerControl({
  threadId,
  facts,
  work,
  permissions,
}: {
  readonly threadId: number;
  readonly facts: WorkFacts;
  readonly work: WorkDetail | undefined;
  readonly permissions: ThreadPermissions;
}) {
  const users = useStore((state) => state.users);
  const owner = <WorkOwner owner={facts.owner} active={facts.ownerActive} size={18} />;

  if (!permissions.canAssignWork || work === undefined) {
    return owner;
  }

  const ownerId = facts.owner?.id ?? null;

  const isAgentCandidate = (candidate: WorkOwnerCandidate) =>
    candidate.provider !== null || isAgent(users[candidate.userId]);

  const people = work.ownerCandidates.filter((candidate) => !isAgentCandidate(candidate));
  const agents = work.ownerCandidates.filter(isAgentCandidate);

  const assign = (userId: number | null) => {
    if (userId !== ownerId) {
      actions.work.assign(threadId, userId).catch(failed("Couldn't change the owner"));
    }
  };

  const option = (candidate: WorkOwnerCandidate) => (
    <OwnerOption
      key={candidate.userId}
      candidate={candidate}
      user={users[candidate.userId]}
      checked={candidate.userId === ownerId}
      onSelect={() => assign(candidate.userId)}
    />
  );

  return (
    <Menu
      label="Owner"
      trigger={(props) => (
        <Button
          {...props}
          variant="ghost"
          size="sm"
          trailingIcon="chevron-down"
          className="work-control"
          aria-label={`Owner: ${facts.owner?.name ?? UNASSIGNED}${
            facts.owner !== null && !facts.ownerActive ? ", inactive" : ""
          }. Change owner`}
        >
          {owner}
        </Button>
      )}
    >
      {people.length === 0 ? null : <MenuGroup label="People">{people.map(option)}</MenuGroup>}
      {agents.length === 0 ? null : <MenuGroup label="Agents">{agents.map(option)}</MenuGroup>}
      {ownerId === null ? null : (
        <>
          {people.length + agents.length === 0 ? null : <MenuSeparator />}
          <MenuItem icon="x" onSelect={() => assign(null)}>
            Unassign
          </MenuItem>
        </>
      )}
    </Menu>
  );
}

/** Confirms "Stop tracking": the thread becomes ordinary again, its owner cleared. */
function StopTrackingDialog({
  threadId,
  open,
  onOpenChange,
}: {
  readonly threadId: number;
  readonly open: boolean;
  readonly onOpenChange: (open: boolean) => void;
}) {
  const [busy, setBusy] = useState(false);

  const stop = () => {
    setBusy(true);
    actions.work.setStatus(threadId, null).then(
      () => {
        setBusy(false);
        onOpenChange(false);
      },
      (error: Error) => {
        setBusy(false);
        onOpenChange(false);
        failed("Couldn't stop tracking the work")(error);
      },
    );
  };

  return (
    <Dialog
      open={open}
      onOpenChange={onOpenChange}
      role="alertdialog"
      size="sm"
      title="Stop tracking this work?"
      description="It goes back to being an ordinary thread and its owner is cleared. Its work history is kept."
      footer={
        <>
          <Button variant="secondary" data-autofocus onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button variant="danger" loading={busy} onClick={stop}>
            Stop tracking
          </Button>
        </>
      }
    />
  );
}

/**
 * The work section in the pane's toolbar, for a tracked thread. What it offers follows the
 * viewer's permissions: status (`canUpdateWorkStatus`), owner (`canAssignWork`), stop tracking
 * (`canRemoveWork`), and the result and handoff (`canManageWork`). Links need none of those:
 * anyone who can see the work may edit them.
 */
export function WorkBar({ threadId }: { readonly threadId: number }) {
  const facts = useStore((state) => state.threads[threadId]?.work ?? null);
  const name = useStore((state) => threadTitle(state.threads[threadId]));
  const work = useStore((state) => state.work.details[threadId]);
  const permissions = useStore((state) => state.threadPanes[threadId]?.permissions ?? null);
  const [stopping, setStopping] = useState(false);
  const { open: handingOff, openHandoff, closeHandoff } = useHandoffRoute(threadId);
  const { open: editingLinks, roomId, openLinks } = useLinksRoute(threadId);
  const { announce, region } = useAnnouncer();

  if (facts === null || permissions === null) {
    return null;
  }

  const canHandOff =
    work !== undefined &&
    handoffRefusal({
      tracked: true,
      canManage: permissions.canManageWork,
      receiverCount: work.handoffReceivers.length,
    }) === null;

  return (
    <section className="work-bar" aria-label="Work">
      <div className="work-bar-facts">
        <StatusControl
          threadId={threadId}
          facts={facts}
          permissions={permissions}
          onStop={() => setStopping(true)}
        />
        <OwnerControl threadId={threadId} facts={facts} work={work} permissions={permissions} />
        <div className="work-bar-links-slot">
          <WorkLinks links={facts.links} runUrl={facts.runUrl} label={`Links for ${name}`} />
          {editingLinks ? null : (
            <Button variant="ghost" size="sm" icon="plus" onClick={openLinks}>
              Link
            </Button>
          )}
        </div>
      </div>
      {editingLinks && roomId !== null ? (
        <div className="work-bar-editor">
          <PostLinks
            // Another thread's draft starts afresh: no busy change carries over.
            key={threadId}
            threadId={threadId}
            roomId={roomId}
            links={facts.links}
            editable
          />
        </div>
      ) : null}
      {work === undefined ? null : (
        <div className="work-bar-details">
          <Accordion title="Result, steps and history">
            <div className="work-details">
              <WorkResult
                threadId={threadId}
                work={work}
                updatedAt={facts.resultUpdatedAt}
                canEdit={permissions.canManageWork}
                announce={announce}
              />
              <WorkStepsSection threadId={threadId} work={work} />
              <WorkHistory threadId={threadId} history={work.history} />
              {canHandOff ? (
                <div className="work-handoff-row">
                  <Button variant="secondary" size="sm" icon="send" onClick={openHandoff}>
                    Hand off to an agent
                  </Button>
                </div>
              ) : null}
            </div>
          </Accordion>
          <HandoffDialog
            threadId={threadId}
            threadName={name}
            work={work}
            open={handingOff && canHandOff}
            onOpenChange={(next) => {
              if (!next) closeHandoff();
            }}
          />
        </div>
      )}
      <StopTrackingDialog threadId={threadId} open={stopping} onOpenChange={setStopping} />
      {region}
    </section>
  );
}

/** "Status: In progress", "Owner: Maya", "No longer tracked as work". */
function changeText(before: WorkFacts | null, after: WorkFacts | null): string | null {
  if (before === null) {
    return after === null ? null : `Tracked as work: ${workStatusLabel(after.status)}`;
  }

  if (after === null) {
    return "No longer tracked as work";
  }

  const parts: string[] = [];

  if (before.status !== after.status) {
    parts.push(`Status: ${workStatusLabel(after.status)}`);
  }

  if ((before.owner?.id ?? null) !== (after.owner?.id ?? null)) {
    parts.push(`Owner: ${after.owner?.name ?? UNASSIGNED}`);
  }

  return parts.length === 0 ? null : parts.join(". ");
}

/**
 * Always in the pane while it shows a thread: says status and owner changes (the viewer's own and
 * live ones) through a polite live region, refetches the detail when `thread.updated` brings
 * facts that differ from the ones it came with, and puts focus on "Thread actions" when stopping
 * tracking took the focused control away.
 */
export function WorkLive({ threadId }: { readonly threadId: number }) {
  const facts = useStore((state) => state.threads[threadId]?.work ?? null);
  const stale = useStore((state) => workDetailStale(state, threadId));
  const { announce, region } = useAnnouncer();
  const anchorRef = useRef<HTMLSpanElement | null>(null);

  const previous = useRef<{ readonly threadId: number; readonly facts: WorkFacts | null } | null>(
    null,
  );

  useEffect(() => {
    if (stale) {
      void actions.work.refresh(threadId);
    }
  }, [stale, threadId]);

  useEffect(() => {
    const last = previous.current;

    previous.current = { threadId, facts };

    if (last === null || last.threadId !== threadId) {
      return;
    }

    const text = changeText(last.facts, facts);

    if (text !== null) {
      announce(text);
    }

    const lostFocus = document.activeElement === null || document.activeElement === document.body;

    if (last.facts !== null && facts === null && lostFocus) {
      anchorRef.current
        ?.closest(".pane-frame")
        ?.querySelector<HTMLElement>('[aria-label="Thread actions"]')
        ?.focus();
    }
  }, [threadId, facts, announce]);

  return <span ref={anchorRef}>{region}</span>;
}

/**
 * "Track as work" in the thread's menu, for whoever may convert it (`canConvertWork`): the
 * thread starts as Planned and unassigned, and the work section appears (`WorkLive` says so).
 */
export function TrackAsWorkItem({ threadId }: { readonly threadId: number }) {
  return (
    <MenuItem
      icon="briefcase"
      onSelect={() =>
        void actions.work
          .setStatus(threadId, "planned")
          .catch(failed("Couldn't track the thread as work"))
      }
    >
      Track as work
    </MenuItem>
  );
}
