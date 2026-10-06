import { type ReactNode, useEffect, useState } from "react";
import { AnimatedNumber } from "../../motion/animated-number.tsx";
import { SuccessCheck } from "../../motion/success-check.tsx";
import { TextSwap } from "../../motion/text-swap.tsx";
import { Accordion } from "../../ui/accordion.tsx";
import { AgentAvatar } from "../../ui/agent-avatar.tsx";
import { AgentThinking, type AgentThinkingState } from "../../ui/agent-thinking.tsx";
import { Avatar, type PresenceStatus } from "../../ui/avatar.tsx";
import { Badge, type BadgeTone } from "../../ui/badge.tsx";
import { Beam } from "../../ui/beam.tsx";
import { Button, type ButtonSize, type ButtonVariant } from "../../ui/button.tsx";
import { Checkbox } from "../../ui/checkbox.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { IconButton } from "../../ui/icon-button.tsx";
import { Icon } from "../../ui/icons/icon.tsx";
import { Kbd } from "../../ui/kbd.tsx";
import {
  Menu,
  MenuCheckboxItem,
  MenuGroup,
  MenuItem,
  MenuSeparator,
  SubMenu,
} from "../../ui/menu.tsx";
import { Popover } from "../../ui/popover.tsx";
import { Skeleton, SkeletonReveal } from "../../ui/skeleton.tsx";
import { SpeakingRing } from "../../ui/speaking-ring.tsx";
import { Tabs } from "../../ui/tabs.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { toast } from "../../ui/toast-store.ts";
import { Toggle } from "../../ui/toggle.tsx";

function Section({
  title,
  note,
  children,
}: {
  readonly title: string;
  readonly note?: string;
  readonly children: ReactNode;
}) {
  return (
    <section className="ks-section">
      <header className="ks-section-header">
        <h2 className="ks-section-title">{title}</h2>
        {note === undefined ? null : <p className="ks-section-note">{note}</p>}
      </header>
      {children}
    </section>
  );
}

function Row({ label, children }: { readonly label?: string; readonly children: ReactNode }) {
  return (
    <div className="ks-row">
      {label === undefined ? null : <span className="ks-row-label">{label}</span>}
      <div className="ks-row-items">{children}</div>
    </div>
  );
}

const VARIANTS: readonly ButtonVariant[] = [
  "primary",
  "secondary",
  "ghost",
  "danger",
  "pill",
  "link",
  "metal",
];

const SIZES: readonly ButtonSize[] = ["sm", "md", "lg"];

function Buttons() {
  const [saving, setSaving] = useState(false);
  const [pressed, setPressed] = useState(true);

  const save = () => {
    setSaving(true);
    window.setTimeout(() => setSaving(false), 1600);
  };

  return (
    <Section title="Button" note="Primary, secondary, ghost, danger, icon, pill, link and metal.">
      {VARIANTS.map((variant) => (
        <Row key={variant} label={variant}>
          {SIZES.map((size) => (
            <Button key={size} variant={variant} size={size}>
              {variant === "metal" ? "Go live" : `Button ${size}`}
            </Button>
          ))}
          <Button variant={variant} icon="plus">
            With icon
          </Button>
          <Button variant={variant} disabled>
            Disabled
          </Button>
          <Button variant={variant} loading loadingLabel="Saving">
            Loading
          </Button>
        </Row>
      ))}
      <Row label="loading swap">
        <Button variant="primary" loading={saving} onClick={save} icon="send">
          Send message
        </Button>
        <Button variant="secondary" loading={saving} loadingLabel="Saving…" onClick={save}>
          Save changes
        </Button>
        <Button variant="pill" aria-pressed={pressed} onClick={() => setPressed(!pressed)}>
          Unreads only
        </Button>
      </Row>
      <Row label="icon">
        <IconButton icon="smile-plus" label="Add reaction" size="sm" />
        <IconButton icon="thread" label="Reply in thread" shortcut={["T"]} />
        <IconButton icon="search" label="Search" shortcut={["⌘", "K"]} size="lg" />
        <IconButton icon="settings" label="Settings" disabled />
        <IconButton icon="bell" label="Notifications" loading />
      </Row>
    </Section>
  );
}

function Menus() {
  const [unreads, setUnreads] = useState(true);
  const [threads, setThreads] = useState(false);

  return (
    <Section
      title="Menu"
      note="Arrow keys, Home/End, typeahead, Enter, Esc, Tab; submenus on ArrowRight or hover."
    >
      <Row>
        <Menu
          trigger={(props) => (
            <Button {...props} trailingIcon="chevron-down">
              Message actions
            </Button>
          )}
        >
          <MenuItem icon="smile-plus" shortcut={["R"]}>
            Add reaction
          </MenuItem>
          <MenuItem icon="thread" shortcut={["T"]}>
            Reply in thread
          </MenuItem>
          <MenuItem icon="forward">Forward</MenuItem>
          <SubMenu label="Remind me" icon="bell">
            <MenuItem>In 20 minutes</MenuItem>
            <MenuItem>In 1 hour</MenuItem>
            <MenuItem>Tomorrow</MenuItem>
            <MenuSeparator />
            <MenuItem>Custom…</MenuItem>
          </SubMenu>
          <MenuItem icon="link">Copy link</MenuItem>
          <MenuItem icon="pin" disabled>
            Pin to channel
          </MenuItem>
          <MenuSeparator />
          <MenuItem icon="pencil" shortcut={["E"]}>
            Edit message
          </MenuItem>
          <MenuItem icon="trash" tone="danger">
            Delete message
          </MenuItem>
        </Menu>
        <Menu
          placement="bottom-end"
          trigger={(props) => <IconButton {...props} icon="more" label="View options" />}
        >
          <MenuGroup label="Show">
            <MenuCheckboxItem checked={unreads} onCheckedChange={setUnreads}>
              Unreads first
            </MenuCheckboxItem>
            <MenuCheckboxItem checked={threads} onCheckedChange={setThreads}>
              Threads in sidebar
            </MenuCheckboxItem>
          </MenuGroup>
          <MenuSeparator />
          <MenuItem icon="settings">Sidebar settings</MenuItem>
        </Menu>
      </Row>
    </Section>
  );
}

function Dialogs() {
  const [open, setOpen] = useState(false);
  const [confirm, setConfirm] = useState(false);
  const [name, setName] = useState("design-system");

  return (
    <Section title="Dialog" note="Native <dialog>: focus trap, Esc, backdrop click, focus return.">
      <Row>
        <Button onClick={() => setOpen(true)}>Rename channel</Button>
        <Button variant="danger" onClick={() => setConfirm(true)}>
          Delete channel
        </Button>
      </Row>
      <Dialog
        open={open}
        onOpenChange={setOpen}
        title="Rename channel"
        description="Names are lowercase, without spaces or periods."
        footer={
          <>
            <Button variant="ghost" onClick={() => setOpen(false)}>
              Cancel
            </Button>
            <Button variant="primary" onClick={() => setOpen(false)}>
              Save
            </Button>
          </>
        }
      >
        <TextField
          label="Name"
          value={name}
          onChange={(event) => setName(event.target.value)}
          data-autofocus=""
        />
      </Dialog>
      <Dialog
        open={confirm}
        onOpenChange={setConfirm}
        role="alertdialog"
        size="sm"
        title="Delete #design-system?"
        description="Everyone loses access to its messages and files. This can't be undone."
        footer={
          <>
            <Button variant="secondary" onClick={() => setConfirm(false)} data-autofocus="">
              Cancel
            </Button>
            <Button variant="danger" onClick={() => setConfirm(false)}>
              Delete channel
            </Button>
          </>
        }
      />
    </Section>
  );
}

function Popovers() {
  return (
    <Section title="Popover" note="Non-modal, top layer, anchored; Esc returns focus.">
      <Row>
        <Popover
          label="Riel St. Amand"
          trigger={(props) => (
            <Button {...props} variant="ghost">
              <Avatar name="Riel St. Amand" size={20} decorative /> Riel St. Amand
            </Button>
          )}
        >
          {(close) => (
            <div className="ks-profile">
              <Avatar name="Riel St. Amand" size={56} presence="online" />
              <div>
                <p className="ks-profile-name">Riel St. Amand</p>
                <p className="text-muted text-meta">Lead · Columbus, 9:41 AM local</p>
              </div>
              <div className="ks-profile-actions">
                <Button variant="primary" size="sm" icon="dms" onClick={close}>
                  Message
                </Button>
                <Button size="sm" icon="phone" onClick={close}>
                  Huddle
                </Button>
              </div>
            </div>
          )}
        </Popover>
      </Row>
    </Section>
  );
}

const PRESENCES: readonly PresenceStatus[] = ["online", "away", "dnd", "offline"];

const PEOPLE = [
  "Ada Lovelace",
  "Grace Hopper",
  "Katherine Johnson",
  "Linus Torvalds",
  "Margaret Hamilton",
];

function Avatars() {
  return (
    <Section
      title="Avatar"
      note="Rounded squares; deterministic tints (never violet: agents only)."
    >
      <Row label="sizes">
        {[20, 24, 32, 36, 48, 80].map((size) => (
          <Avatar key={size} name="Grace Hopper" size={size} />
        ))}
      </Row>
      <Row label="presence">
        {PRESENCES.map((presence, index) => (
          <Avatar key={presence} name={PEOPLE[index] ?? "Ada"} presence={presence} />
        ))}
      </Row>
      <Row label="tints">
        {PEOPLE.map((person) => (
          <Avatar key={person} name={person} size={32} />
        ))}
      </Row>
    </Section>
  );
}

const TONES: readonly BadgeTone[] = ["danger", "accent", "mention", "neutral"];

function Badges() {
  const [count, setCount] = useState(3);

  return (
    <Section title="Badge" note="Pops in on a spring; digits tick when the count changes.">
      <Row label="tones">
        {TONES.map((tone) => (
          <Badge key={tone} count={count} tone={tone} />
        ))}
        <Badge count={128} />
        <Badge count={1} dot />
      </Row>
      <Row label="count">
        <Button size="sm" onClick={() => setCount(count + 1)} icon="plus">
          Add
        </Button>
        <Button size="sm" onClick={() => setCount(Math.max(0, count - 1))}>
          Remove
        </Button>
        <Button size="sm" variant="ghost" onClick={() => setCount(count === 0 ? 7 : 0)}>
          {count === 0 ? "Show" : "Clear"}
        </Button>
        <span className="ks-floating-demo">
          <Icon name="inbox" size={20} />
          <Badge count={count} floating />
        </span>
      </Row>
    </Section>
  );
}

function KeysAndTabs() {
  const [tab, setTab] = useState("messages");

  return (
    <Section title="Kbd and Tabs">
      <Row label="kbd">
        <Kbd keys={["⌘", "K"]} />
        <Kbd keys={["⌘", "⇧", "A"]} />
        <Kbd keys={["Esc"]} />
        <Kbd keys={["↑"]} />
      </Row>
      <Tabs
        label="Search results"
        value={tab}
        onValueChange={setTab}
        items={[
          { value: "messages", label: "Messages", icon: "dms" },
          { value: "files", label: "Files" },
          { value: "channels", label: "Channels", icon: "hash" },
          { value: "people", label: "People", icon: "users" },
        ]}
      >
        <p className="text-muted text-ui">Showing {tab}.</p>
      </Tabs>
    </Section>
  );
}

function allState(checks: readonly boolean[]): boolean | "mixed" {
  if (checks.every(Boolean)) {
    return true;
  }

  return checks.some(Boolean) ? "mixed" : false;
}

function Controls() {
  const [notify, setNotify] = useState(true);
  const [sounds, setSounds] = useState(false);
  const [checks, setChecks] = useState([true, false]);
  const all = allState(checks);

  return (
    <Section title="Toggle and Checkbox">
      <div className="ks-stack ks-narrow">
        <Toggle
          checked={notify}
          onCheckedChange={setNotify}
          label="Desktop notifications"
          description="Mentions, DMs and keywords."
        />
        <Toggle checked={sounds} onCheckedChange={setSounds} label="Message sounds" />
        <Toggle checked onCheckedChange={() => {}} label="Managed by your admin" disabled />
      </div>
      <div className="ks-stack">
        <Checkbox
          checked={all}
          onCheckedChange={(value) => setChecks([value, value])}
          label="All channels"
        />
        <div className="ks-indent ks-stack">
          <Checkbox
            checked={checks[0] === true}
            onCheckedChange={(value) => setChecks([value, checks[1] === true])}
            label="#general"
          />
          <Checkbox
            checked={checks[1] === true}
            onCheckedChange={(value) => setChecks([checks[0] === true, value])}
            label="#design"
          />
        </div>
        <Checkbox checked={false} onCheckedChange={() => {}} label="Disabled" disabled />
      </div>
    </Section>
  );
}

function Loading() {
  const [loading, setLoading] = useState(true);

  return (
    <Section title="Skeleton" note="The reveal blurs the skeleton away as the content sharpens in.">
      <Row>
        <Button size="sm" onClick={() => setLoading(!loading)}>
          {loading ? "Reveal" : "Reload"}
        </Button>
      </Row>
      <SkeletonReveal
        loading={loading}
        skeleton={
          <div className="ks-message-skeleton">
            <Skeleton width={36} height={36} radius="md" />
            <div className="ks-stack-tight">
              <Skeleton width={140} height={12} />
              <Skeleton width="92%" height={12} />
              <Skeleton width="64%" height={12} />
            </div>
          </div>
        }
      >
        <div className="ks-message-skeleton">
          <Avatar name="Katherine Johnson" decorative />
          <div>
            <p className="text-ui">
              <strong>Katherine Johnson</strong> <span className="text-faint text-meta">10:24</span>
            </p>
            <p className="text-body">
              Trajectory numbers are in the sheet. The launch window holds if the weather does.
            </p>
          </div>
        </div>
      </SkeletonReveal>
    </Section>
  );
}

function Toasts() {
  return (
    <Section
      title="Toast"
      note="Bottom-left stack; hover or focus fans it out and pauses the timers."
    >
      <Row>
        <Button onClick={() => toast({ title: "Link copied" })}>Neutral</Button>
        <Button
          onClick={() =>
            toast({ title: "Message sent", description: "Delivered to #design.", tone: "success" })
          }
        >
          Success
        </Button>
        <Button
          onClick={() =>
            toast({
              title: "Couldn't upload brief.pdf",
              description: "The file is over 100 MB.",
              tone: "danger",
              action: { label: "Retry", onClick: () => {} },
            })
          }
        >
          Danger with action
        </Button>
      </Row>
    </Section>
  );
}

function Fields() {
  const [value, setValue] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);

  return (
    <Section title="Text field" note="Invalid submit shakes the field once.">
      <form
        className="ks-form"
        onSubmit={(event) => {
          event.preventDefault();
          setAttempt(attempt + 1);
          setError(
            value.includes("@") ? undefined : "Enter an email address, like ada@example.com.",
          );
        }}
      >
        <TextField
          label="Invite by email"
          placeholder="name@company.com"
          hint="They'll get a link to join this workspace."
          value={value}
          error={error}
          attempt={attempt}
          onChange={(event) => setValue(event.target.value)}
        />
        <Button type="submit" variant="primary">
          Send invite
        </Button>
      </form>
    </Section>
  );
}

function Accordions() {
  return (
    <Section title="Accordion">
      <div className="ks-narrow">
        <Accordion title="Channels" defaultOpen>
          <p className="text-muted text-ui">#general, #design, #ops</p>
        </Accordion>
        <Accordion title="Direct messages">
          <p className="text-muted text-ui">Ada, Grace, Katherine</p>
        </Accordion>
      </div>
    </Section>
  );
}

const STATUSES = ["Send", "Sending…", "Sent"] as const;

function Motion() {
  const [status, setStatus] = useState(0);
  const [value, setValue] = useState(1284);
  const [copied, setCopied] = useState(false);
  const [done, setDone] = useState(false);

  return (
    <Section title="Motion" note="Text swap, number pop-in, icon swap and success check.">
      <Row label="text swap">
        <Button variant="secondary" onClick={() => setStatus((status + 1) % STATUSES.length)}>
          <TextSwap reserve={STATUSES}>{STATUSES[status] ?? "Send"}</TextSwap>
        </Button>
      </Row>
      <Row label="number">
        <span className="ks-number">
          <AnimatedNumber value={value} />
        </span>
        <Button size="sm" onClick={() => setValue(value + Math.ceil(Math.random() * 40))}>
          Up
        </Button>
        <Button
          size="sm"
          onClick={() => setValue(Math.max(0, value - Math.ceil(Math.random() * 40)))}
        >
          Down
        </Button>
      </Row>
      <Row label="icon swap">
        <Button
          variant="secondary"
          onClick={() => {
            setCopied(true);
            window.setTimeout(() => setCopied(false), 1400);
          }}
        >
          <span className="t-icon-swap" data-state={copied ? "b" : "a"}>
            <span className="t-icon" data-icon="a">
              <Icon name="copy" />
            </span>
            <span className="t-icon" data-icon="b">
              <Icon name="check" />
            </span>
          </span>
          Copy link
        </Button>
      </Row>
      <Row label="success">
        <Button variant="secondary" onClick={() => setDone(!done)}>
          {done ? "Reset" : "Complete"}
        </Button>
        <span className="ks-success">{done ? <SuccessCheck size={20} /> : null}</span>
      </Row>
    </Section>
  );
}

const ORB_STATES: readonly AgentThinkingState[] = [
  "working",
  "searching",
  "solving",
  "composing",
  "connecting",
];

function useVoiceLevel(active: boolean): number {
  const [level, setLevel] = useState(0);

  useEffect(() => {
    if (!active) {
      return;
    }

    let frame = 0;

    const tick = (time: number) => {
      setLevel(0.35 + 0.35 * Math.sin(time / 180) + 0.25 * Math.sin(time / 67));
      frame = requestAnimationFrame(tick);
    };

    frame = requestAnimationFrame(tick);

    return () => cancelAnimationFrame(frame);
  }, [active]);

  return active ? level : 0;
}

function Effects() {
  const [beam, setBeam] = useState<"a" | "b" | "none">("a");
  const [speaking, setSpeaking] = useState(true);
  const level = useVoiceLevel(speaking);

  return (
    <Section
      title="Agents and voice"
      note="Jakub Antalik's thinking-orbs, border-beam, voice-glow and bot-avatars, code-split."
    >
      <Row label="thinking">
        {ORB_STATES.map((state) => (
          <AgentThinking key={state} size={32} state={state} label={`Agent ${state}`} />
        ))}
        <AgentThinking size={20} />
        <AgentThinking size={64} state="composing" />
      </Row>
      <Row label="bots">
        {["agent-1", "agent-2", "agent-3", "agent-4", "agent-5", "agent-6"].map((seed) => (
          <AgentAvatar key={seed} seed={seed} name={`Bot ${seed}`} />
        ))}
        <AgentAvatar seed="agent-7" name="Scout" size={80} />
      </Row>
      <Row label="beam">
        <Beam active={beam === "a"}>
          <button type="button" className="ks-card" onClick={() => setBeam("a")}>
            <AgentThinking size={20} />
            Scout is drafting a summary
          </button>
        </Beam>
        <Beam active={beam === "b"}>
          <button type="button" className="ks-card" onClick={() => setBeam("b")}>
            <AgentThinking size={20} state="searching" />
            Muse is searching files
          </button>
        </Beam>
        <Button size="sm" variant="ghost" onClick={() => setBeam("none")}>
          Stop
        </Button>
      </Row>
      <Row label="speaking">
        <SpeakingRing level={level} speaking={speaking} radius={12}>
          <Avatar name="Grace Hopper" size={48} decorative />
        </SpeakingRing>
        <SpeakingRing level={0} speaking={false} radius={12}>
          <Avatar name="Ada Lovelace" size={48} decorative />
        </SpeakingRing>
        <Button
          size="sm"
          icon={speaking ? "mic-off" : "mic"}
          onClick={() => setSpeaking(!speaking)}
        >
          {speaking ? "Mute" : "Unmute"}
        </Button>
      </Row>
    </Section>
  );
}

/** Every component in every variant and state. */
export function Gallery() {
  return (
    <div className="ks-gallery">
      <Buttons />
      <Menus />
      <Dialogs />
      <Popovers />
      <Avatars />
      <Badges />
      <KeysAndTabs />
      <Controls />
      <Loading />
      <Toasts />
      <Fields />
      <Accordions />
      <Motion />
      <Effects />
    </div>
  );
}
