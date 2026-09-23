# Slash commands

Typing `/` at the start of the composer opens a command picker, Discord
and Slack style. Picking a command inserts `/name ` so arguments can be
typed after it; submitting runs it. Commands that post (shrug, me, play,
remind) broadcast live like typed messages; the rest answer ephemerally
— visible to the invoker only, never posted — or open UI.

Commands live in one registry
(`SlashCommands::Registry`, handlers in `SlashCommands::Handlers`):
each entry has a name, a description, an argument hint, a permission
check, and a handler. Adding a command means adding one entry plus one
handler method. `/play` is a registry entry that posts through the
normal message path, so sounds keep their optimistic preview and mute
rules; see [status and notifications](notifications.md).

## Built-in commands

- `/huddle` starts a call in this room. It needs configured huddles;
  otherwise it answers an ephemeral error.
- `/event <title> <when>` opens the event form with the title (and the
  time, when one parses) prefilled. For example
  `/event Launch party friday 5pm`.
- `/poll` opens the poll builder. Channel only, not threads.
- `/remind <when> <text>` posts the text and saves it for you with a
  reminder at `<when>`, using the saved-items feature; see
  [pins and saved items](pins-and-saved.md). For example
  `/remind in 20 minutes review the deploy`.
- `/status <emoji> <text>` sets your custom status until the end of
  today in your time zone. For example `/status 🚂 On a train`.
- `/dnd [duration]` toggles Do Not Disturb. Bare `/dnd` flips the
  switch; `/dnd 30m`, `/dnd 2h`, and `/dnd until 5pm` turn it on until
  then; `/dnd off` turns it off. Timed DND expires lazily like custom
  statuses do — no cleanup job.
- `/ooo <duration or date> [note]` sets [out of
  office](out-of-office.md) until then with an optional note. For
  example `/ooo tomorrow Back soon` or `/ooo 1 week`. `/ooo off`
  clears the manual OOO; a calendar OOO keeps showing if one covers
  you.
- `/shrug [text]` posts the text (if any) with `¯\_(ツ)_/¯`.
- `/me <action>` posts an action line ("David is reviewing the
  deploy"), rendered in italics.

Times resolve in the invoker's own time zone and accept `in 20
minutes`, `in 1 hour`, `tomorrow at 9am`, `today at 3pm`, `at 15:00`,
`friday 5pm`, `next friday`, and explicit datetimes like
`2026-10-01 15:00`. A bare weekday means its next occurrence at 9am.

A leading `/word` the composer doesn't recognize posts as a normal
message, so "/etc/hosts" never errors — only known commands
(built-ins and commands registered in the room) run. The composer
re-checks the live list for words it doesn't know, so a command
registered after the page loaded still runs. To post a literal line
starting with a known command, escape it with `//`. Direct calls to
the slash endpoint still answer unknown commands with an ephemeral
error instead of posting.

## Agent commands

Agents register custom slash commands per room. Invoking one delivers
a `slash_command` event to the owning agent with the raw arguments —
plus the thread id when invoked in a thread, so the agent can reply
in place — and answers ephemerally ("Sent to \<agent\>") until the
agent replies.
Registration and invocation both require the agent to hold
`post_messages` in that room; a revoked agent's commands answer
"no longer available". Reading the events keeps the standard
`read_messages` polling gate like every other event type. Names are
lowercase, unique per room across agents, and cannot shadow built-ins.
See [AI agents](agents.md#slash-commands).
