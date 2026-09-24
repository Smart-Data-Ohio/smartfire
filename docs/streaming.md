# Streaming on stage channels

A stream is a stage channel gone live: a host or speaker presents their
screen at an explicit quality, every other member sees a Live indicator and
can watch, and viewers get a quality control. The presenter is the stage
host or speaker who is live; everyone else in the call is a viewer. Streams
build on the ordinary huddle screen share — the same capture, the same
expanded-screen layout — with explicit presenter/viewer behavior around it.
See [stage channels](stage-channels.md) for roles and [huddles](huddles.md)
for the media foundation.

## Going live

Hosts and speakers see a **Go live** form in the stage panel: a quality
select (`720p15`, `1080p15`, `1080p30`, defaulting to `1080p15`) and a
button. The click hands the whole sequence to the huddle panel, which
captures the screen inside the click gesture first — Safari denies a
capture that starts after the POST round-trip — then posts the stream
(`POST /rooms/:room_id/stage/stream` with a `quality` parameter) and
publishes the captured tracks at that quality. Denying the capture posts
nothing; a failed POST stops the tracks. A host or speaker can also
share through the ordinary Share screen control; that share carries the
room default and never marks the room live. The Share control hides where
`getDisplayMedia` is missing.

One room carries at most one live stream. Starting while another stream is
live answers 409 and names the presenter. Listeners cannot go live, and
non-members get 404 like the other stage endpoints.

Going live requires joining the stage call first: the Go live control stays disabled until the huddle connects to the room, and starting without an active huddle grant is rejected, so a stream never goes live with nothing to publish over.

## Watching

While a room is live, the room header shows a `Live` badge with the
presenter's name next to the stage join button, the sidebar row shows a
small live dot, and the stage panel shows "Live: NAME". All three update
over Turbo Streams when a stream starts or ends.

Viewers watching from the room page get the presenter's share expanded
into theater mode automatically, on joining late as well as when the
stream starts mid-call. The expanded share carries an "N watching" count:
everyone in the call minus the presenter. A viewer quality control offers
`Auto`, `Low`, and `High`, applied to the presenter's screen-share
subscription; `Auto` clears the explicit request so adaptive streaming
sizes the share from the rendered element again. The choice is remembered
per browser in `localStorage` under `campfire.huddle.streamQuality`.

## Quality presets

The presenter's quality maps onto the LiveKit screen-share presets:

- `720p15`: 1280×720 at up to 1.5 Mbps and 15 fps.
- `1080p15`: 1920×1080 at up to 2.5 Mbps and 15 fps. The default, and the
  same ceiling as an ordinary share.
- `1080p30`: 1920×1080 at up to 5 Mbps and 30 fps. Roughly double the
  uplink of the default; pick it for motion, not for slides.

Like the ordinary share, capture stays at the SDK's own 1080p/30 while the
encoder applies the preset ceiling, and congestion drops frames before
resolution so shared text stays readable. These are requested encoding
settings, not measured output; see the [quality assessment](huddle-quality.md)
for what is and is not measured.

## Stopping and automatic ends

The stage panel shows **Stop stream** to the presenter and to hosts.
Stopping (`DELETE /rooms/:room_id/stage/stream`) ends the live state; the
presenter's browser stops sharing alongside it. An administrator member
can also stop through the endpoint. Anyone else gets 403. Both the Stop
control and the presenting browser name the stream id they mean to stop,
so a delayed stop can never end someone else's newer stream.

A stream also ends, in the same transaction, when:

- the presenter is demoted to listener;
- the presenter's membership is removed;
- the presenter's user is deactivated;
- the presenter's last active grant for the room is revoked, including
  through the gateway's authorization check. Only a change that crosses
  the publish boundary (to or from listener) revokes grants; a host↔speaker
  change keeps the grant and the stream it carries.

Ending always broadcasts the same updates as an explicit stop. On the
presenting browser, a cancelled or denied capture, the browser's own stop
control, and leaving the call end the stream as well — the leaving call
uses `keepalive` so it survives tab close — and the huddle reconciler ends
any live stream whose presenter has had no in-call grant for thirty
seconds, so no live state dangles behind a share that is already gone.
When a host stops someone else's stream, the presenter's browser is notified through the huddle panel and stops sharing, instead of the share continuing as an ordinary screen share.

When the last host membership disappears — the host leaves or is removed,
or their user is deactivated — the whole live session ends, not just one
stream: every live stream in the room ends and every active huddle grant in
the room is revoked, dropping the remaining members from the call. Nobody
can go live again until a host exists; an administrator member promotes a
new host from the stage panel's role controls.

## Deliberately not included

Recording, an external or public audience, and RTMP are not part of
streaming. A stream is visible to room members only, it is never recorded,
and it never leaves the room's call.
