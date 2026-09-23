# Polls

A poll is a question with 2–10 options posted as a message. Open the
builder with `/poll`, write the question and options (blank rows are
dropped), and optionally allow multiple choices, hide voter names, or
set a close time. The question is the message text, so search, quotes,
and edits treat it like any message; the card below it carries the
ballot box.

## Voting

- Single choice (the default) takes one option; multiple choice takes
  several. Everyone can change their vote — or retract it — until the
  poll closes.
- Results update live: every vote replaces the card over the room's
  message stream for all viewers.
- Non-anonymous polls list voter names per option; anonymous polls show
  counts only. Either way one person holds one ballot per poll (per
  option for multiple choice).
- A close time locks voting when it passes; the periodic runner stamps
  closed polls and refreshes their cards within a tick.

Polls live on channel messages, not thread replies. Deleting the
message deletes the poll with it.

## Rendering notes

The card renders inside the shared message fragment, which caches
across viewers in production. Counts, bars, and voter names are
viewer-agnostic and render server-side; the viewer's own checks, voted
markers, and the retract control are marked client-side by the `poll`
Stimulus controller from the card's voter-id data, the same way
reaction chips mark themselves. Anonymous cards omit the voter ids —
they would deanonymize every ballot from view source — and the
controller fetches the viewer's own ballot from the poll endpoint
(`GET /rooms/:room_id/polls/:id`, counts plus per-option voted flags)
instead. Votes and closes touch the poll row, and its stamp rides in
the message cache key and the page etag, so cached fragments and
conditional GETs converge on every ballot.

## Agent API

Agents with `post_messages` create polls and read live results through
the agent API and MCP; see [AI agents](agents.md#polls). Both creation
and reads use the posting grant. Agents cannot vote.
