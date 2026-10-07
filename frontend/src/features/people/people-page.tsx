import { Link, useNavigate, useRouter } from "@tanstack/react-router";
import {
  type ChangeEvent,
  type ReactNode,
  useCallback,
  useEffect,
  useId,
  useRef,
  useState,
} from "react";
import type { DirectoryPerson } from "../../gen/DirectoryPerson.ts";
import { useStore } from "../../store/store.ts";
import { peoplePages } from "../../sync/admin.ts";
import { directs, validationMessage } from "../../sync/directs.ts";
import { Button } from "../../ui/button.tsx";
import { toast } from "../../ui/toast-store.ts";
import { PageFrame } from "../destinations/page-frame.tsx";
import { goJoin } from "../huddle/alerts.ts";
import { PaneError, PaneListSkeleton } from "../panes/pane-states.tsx";
import { UNKNOWN_NAME, useUser } from "./people.ts";
import { botPage, directoryBadge, selectionPlan, toggleSelection } from "./people-format.ts";
import { UserAvatar } from "./user-avatar.tsx";
import "./people.css";

type Load =
  | { readonly status: "loading" }
  | { readonly status: "error"; readonly message: string }
  | { readonly status: "ready"; readonly people: readonly DirectoryPerson[] };

/** Whether the SPA has an agent profile to open (S4); otherwise a bot's page stays classic. */
export function useHasAgentPage(): boolean {
  const router = useRouter();

  return Object.hasOwn(router.routesByPath, "/agents/$agentId");
}

/** The link to someone's page: theirs in the SPA, or for a bot its agent or classic page. */
export function PersonLink({
  userId,
  className,
  children,
}: {
  readonly userId: number;
  readonly className?: string;
  readonly children: ReactNode;
}) {
  const user = useUser(userId);
  const hasAgentPage = useHasAgentPage();

  if (user?.role === "bot") {
    return (
      <a href={botPage(user, hasAgentPage)} className={className}>
        {children}
      </a>
    );
  }

  return (
    <Link to="/people/$userId" params={{ userId }} className={className}>
      {children}
    </Link>
  );
}

/** One directory row: a checkbox, the avatar, the name with whether they're online, the badges. */
function DirectoryRow({
  person,
  checked,
  onToggle,
}: {
  readonly person: DirectoryPerson;
  readonly checked: boolean;
  readonly onToggle: (userId: number, range: boolean) => void;
}) {
  const user = useUser(person.userId);
  const name = user?.name ?? UNKNOWN_NAME;
  const badge = user === undefined ? null : directoryBadge(user, person.agent);

  const change = (event: ChangeEvent<HTMLInputElement>) => {
    const native = event.nativeEvent;

    onToggle(person.userId, native instanceof MouseEvent && native.shiftKey);
  };

  return (
    <li className="people-row" data-user={person.userId} data-selected={checked || undefined}>
      <input
        type="checkbox"
        className="people-check"
        checked={checked}
        onChange={change}
        aria-label={`Select ${name}`}
      />
      <UserAvatar userId={person.userId} size={32} decorative />
      <span className="people-identity">
        <PersonLink userId={person.userId} className="people-name">
          {name}
        </PersonLink>
        <span className="people-presence text-faint">{person.online ? "Online" : "Offline"}</span>
      </span>
      {person.starred ? (
        <span
          className="people-badge"
          role="img"
          aria-label="Starred by you"
          title="Starred by you"
        >
          ★
        </span>
      ) : null}
      {badge === null ? null : <span className="people-badge">{badge}</span>}
    </li>
  );
}

/** The bar under a selection: Message, Start huddle and Clear, with why one is off. */
function SelectionBar({
  selected,
  isBot,
  busy,
  onMessage,
  onHuddle,
  onClear,
}: {
  readonly selected: readonly number[];
  readonly isBot: (userId: number) => boolean;
  readonly busy: boolean;
  readonly onMessage: () => void;
  readonly onHuddle: () => void;
  readonly onClear: () => void;
}) {
  const noteId = useId();
  const plan = selectionPlan(selected, isBot);
  const described = plan.note === "" ? undefined : noteId;

  return (
    <div className="people-bar" hidden={selected.length === 0}>
      <div className="people-bar-actions">
        <Button
          variant="primary"
          icon="message-circle"
          disabled={busy || plan.messageDisabled}
          aria-describedby={described}
          title={plan.messageDisabled ? plan.note : undefined}
          onClick={onMessage}
        >
          {plan.messageLabel}
        </Button>
        <Button
          variant="secondary"
          icon="phone"
          disabled={busy || plan.huddleDisabled}
          aria-describedby={described}
          title={plan.note === "" ? undefined : plan.note}
          onClick={onHuddle}
        >
          {plan.huddleLabel}
        </Button>
        <Button variant="ghost" onClick={onClear}>
          Clear
        </Button>
      </div>
      <p id={noteId} className="people-bar-note text-muted" hidden={plan.note === ""}>
        {plan.note}
      </p>
    </div>
  );
}

/**
 * `/app/people`: everyone in the workspace, as the classic directory lists them (starred first,
 * then by name). Choose people with the checkboxes (Shift extends from the last one) to message
 * them together or start a huddle that rings the people among them.
 */
export function PeoplePage() {
  const navigate = useNavigate();
  const [load, setLoad] = useState<Load>({ status: "loading" });
  const [selected, setSelected] = useState<readonly number[]>([]);
  const [busy, setBusy] = useState(false);
  const anchor = useRef<number | null>(null);
  const users = useStore((state) => state.users);

  const fetchPeople = useCallback(() => {
    peoplePages.directory().then(
      (list) => setLoad({ status: "ready", people: list }),
      (error: Error) => setLoad({ status: "error", message: error.message }),
    );
  }, []);

  useEffect(fetchPeople, [fetchPeople]);

  const list = load.status === "ready" ? load.people : [];
  const order = list.map((person) => person.userId);

  const toggle = (userId: number, range: boolean) => {
    const from = anchor.current;

    anchor.current = userId;
    setSelected((ids) => toggleSelection(ids, userId, order, from, range));
  };

  const clear = () => {
    setSelected([]);
    anchor.current = null;
  };

  const isBot = (userId: number) => users[userId]?.role === "bot";

  const open = (huddle: boolean) => {
    setBusy(true);
    directs.create(selected).then(
      (row) => {
        setBusy(false);
        clear();

        if (huddle) {
          void goJoin(row.room.id, row.displayName);
        } else {
          void navigate({ to: "/r/$roomId", params: { roomId: row.room.id } });
        }
      },
      (error: Error) => {
        // The selection stays for another try, as the classic bar keeps it on a failure.
        setBusy(false);
        toast({
          title: huddle ? "Couldn't start the huddle" : "Couldn't start the conversation",
          description: validationMessage(error) ?? error.message,
          tone: "danger",
        });
      },
    );
  };

  return (
    <PageFrame title="People" icon="users" back>
      <div className="people-page">
        <p className="people-intro text-muted">
          Everyone in the workspace. Select people to message them or start a huddle.
        </p>
        {load.status === "loading" ? <PaneListSkeleton rows={6} /> : null}
        {load.status === "error" ? (
          <PaneError
            message={load.message}
            onRetry={() => {
              setLoad({ status: "loading" });
              fetchPeople();
            }}
          />
        ) : null}
        {load.status === "ready" && list.length === 0 ? (
          <p className="people-empty text-muted">Nobody else is here yet.</p>
        ) : null}
        {list.length > 0 ? (
          <ul className="people-list" aria-label="People">
            {list.map((person) => (
              <DirectoryRow
                key={person.userId}
                person={person}
                checked={selected.includes(person.userId)}
                onToggle={toggle}
              />
            ))}
          </ul>
        ) : null}
      </div>
      <span className="visually-hidden" role="status">
        {selected.length} selected
      </span>
      <SelectionBar
        selected={selected}
        isBot={isBot}
        busy={busy}
        onMessage={() => open(false)}
        onHuddle={() => open(true)}
        onClear={clear}
      />
    </PageFrame>
  );
}
