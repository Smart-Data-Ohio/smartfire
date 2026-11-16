# Email to room

Any chat room (direct and board rooms excluded) can have a secret forward-to address.
Mail sent or forwarded there posts in the room. Mail posts as the matched
active room member when the relay verifies a DKIM signature from the
`From` domain (see [Sender verification](#sender-verification)). Other mail
posts as the workspace **Email** bot with the sender named in the body.

## The address

Each room's address looks like
`room-<random-token>@<configured domain>`. Anyone who can administer the
room (an administrator, or its creator) creates or rotates it from the
room's settings page; rotating retires the old address immediately.
Direct and board rooms never have an address, and deleted rooms receive
nothing.

Keep the address private. Anyone who knows it can post mail to the room.
If the address leaks, rotate it in the room's settings.

While `INBOUND_EMAIL_DOMAIN` is unset, inbound email is disabled cleanly:
the mailbox drops everything, the room settings explain what to set, and
nothing errors. Member verification additionally needs
`INBOUND_EMAIL_AUTHSERV_ID` (see
[Sender verification](#sender-verification)); while it is unset,
everything that arrives posts as the **Email** bot.

## What arrives

- The subject becomes the message's first line, then the body. Mail from
  the **Email** bot is prefixed with `From <sender>`.
- Plain text is preferred. HTML-only mail is sanitized (script and style
  elements removed, the rest run through the safe-list sanitizer) and
  stripped to plain text; the stored message is never raw HTML.
- The first attachment within the 10 MB limit lands on the message
  (messages carry one file); every file is named in the body, and files
  over the limit are named with the reason instead of arriving silently
  missing. The size is estimated from the encoded MIME part before
  decoding, so a huge part is rejected without paying for the decode;
  the estimate allows 5% for transfer-encoding overhead, and the
  decoded size is then checked against the limit exactly.
- Only images, PDFs, text files, and office documents are attached;
  other types are named with `file type not allowed` and never land on
  the message.
- Bodies are capped at the normal message length. Empty mail with no
  attachment posts nothing.
- Each room accepts at most 30 emailed messages per hour (an in-process
  hour bucket, like the agent API throttle); over-limit mail is
  dropped silently.
- Unknown tokens post nothing and bounce nothing, so the address reveals
  nothing to probers. Only room addresses on `INBOUND_EMAIL_DOMAIN`
  count, with a case-insensitive domain match. Mail with no room token
  on that domain is marked bounced instead of raising a routing error.
  No reply is sent, since the app has no outbound mail and answering
  misaddressed mail would backscatter.

## Ingress setup (relay provider)

Production accepts inbound mail through a relay ingress compatible with
Rails Action Mailbox's. The relay endpoint is
`<app root URL>/rails/action_mailbox/relay/inbound_emails`, authenticated
with HTTP basic auth as user `actionmailbox`, and takes the raw message as
the request body with `Content-Type: message/rfc822`.

1. Generate a strong password and give it to the app as the
   `RAILS_INBOUND_EMAIL_PASSWORD` environment variable. Serve the
   app over HTTPS: basic auth over plain HTTP leaks the password.
2. Point the inbound domain's MX records at the relay provider (for
   example Postmark's inbound stream, Mailgun Routes, or SendGrid's
   Inbound Parse) following that provider's domain-verification steps.
   This is DNS/org configuration outside this repo; nothing here changes
   it.
3. Deliver raw RFC 822 bytes to the relay endpoint with basic auth.
   Provider webhooks are not interchangeable with this endpoint. It rejects
   JSON and form payloads with HTTP 415. Before stamping delivery headers,
   the relay or adapter must strip or overwrite all incoming `X-Original-To`,
   `Delivered-To`, and `X-Forwarded-To` fields so sender-supplied or stale
   recipients cannot control routing. Choose one of these paths:
   - **Cloudflare Email Routing + Email Worker:** route the receiving
     subdomain's catch-all to a Worker. Its
     [email handler](https://developers.cloudflare.com/email-service/api/route-emails/email-handler/)
     exposes `message.raw` and the envelope recipient `message.to`.
     Prepend `X-Original-To: <message.to>` to the raw message, then POST
     it with `Content-Type: message/rfc822` and basic auth. Store the
     password as a Worker secret. Handle failed POSTs explicitly.
     Cloudflare [documents a standard `Authentication-Results` header](https://developers.cloudflare.com/email-service/api/route-emails/email-handler/#authentication-results).
     Set `INBOUND_EMAIL_AUTHSERV_ID=mx.cloudflare.net`. The adapter must
     protect that header as described below.
   - **Mailgun:** use an adapter that validates Mailgun's webhook,
     extracts `body-mime` from a
     [MIME route](https://documentation.mailgun.com/docs/mailgun/user-manual/receive-forward-store/receive-http),
     and prepends `X-Original-To` from its `recipient` envelope field.
     The adapter POSTs those raw bytes to Smartfire. Mailgun's HTTP
     route itself posts form data, not `message/rfc822`.
   - **Postmark:** enable raw email content, then use an adapter that
     authenticates the webhook and extracts `RawEmail` and the envelope
     recipient `OriginalRecipient` from its
     [JSON payload](https://postmarkapp.com/developer/webhooks/inbound-webhook).
     Prepend `X-Original-To` with that recipient and POST the raw bytes.
     Enabling raw content still sends JSON, not a raw HTTP body.
   - **SendGrid:** enable "POST the raw, full MIME message", then use an
     adapter that authenticates the webhook, extracts the `email` MIME
     field and recipients from `envelope.to`, and POSTs the raw bytes
     with `X-Original-To` for the intended room. Its
     [Inbound Parse webhook](https://www.twilio.com/docs/sendgrid/api-reference/settings-inbound-parse)
     sends multipart form data, including in raw mode.
   - **Self-hosted Postfix/Exim/Qmail:** pipe inbound mail to an HTTP
     POST of its raw bytes. Preserve the final envelope recipient as
     `X-Original-To` or `Delivered-To`. After adding that header, use
     `curl --fail --data-binary @- -H 'Content-Type: message/rfc822' -u actionmailbox:<password> https://<app host>/rails/action_mailbox/relay/inbound_emails`.
     Keep mail queued when the POST fails. A DKIM/DMARC milter such as
     OpenDKIM, OpenDMARC, or Rspamd can stamp `Authentication-Results`.

   The endpoint receives no SMTP envelope or provider form fields.
   Routing considers only addresses on `INBOUND_EMAIL_DOMAIN`, ignoring
   domain case, in this order: `X-Original-To`, `Delivered-To` (topmost
   first), `X-Forwarded-To`, `To`, `Cc`, then `Bcc`. The first token for an
   existing, non-deleted chat room wins. Unknown tokens and ineligible
   rooms are skipped. An envelope-only delivery needs the relay or
   adapter to add a delivery header. Validate envelope addresses before
   inserting them into a header, including rejecting carriage returns
   and line feeds.

   For member attribution, the relay must stamp its own
   `Authentication-Results` header. The app trusts only the topmost
   matching `INBOUND_EMAIL_AUTHSERV_ID`, with `dkim=pass` and `header.d`
   matching the `From` domain exactly. Provider-specific verdict fields
   are not read. Leave `INBOUND_EMAIL_AUTHSERV_ID` unset unless the relay
   supplies and protects this header. Delivery still works as the **Email** bot.
4. Send a test message to a room's address and confirm it lands in the
   room; rotate the room's address afterwards if the test address was
   shared.

Inbound mail is deduplicated by `(message_id, checksum of the raw bytes)`.
An identical delivery does not post again. Copies with different raw bytes
post separately even when they share a message id. A message Cc'd to the room
and also forwarded from an inbox therefore posts twice. Processing runs
through the normal job backend, so a failed run retries like any other job.

## Forwarding an inbox (Gmail / Google Workspace)

1. Create an address in the room's **Email to room** settings. In the
   Gmail inbox, open **Settings > See all settings > Forwarding and
   POP/IMAP**, then **Add a forwarding address** and enter the room address.
2. Gmail sends a confirmation email to the room. Open its verification
   link or copy its confirmation code back into Gmail. This email posts
   as the **Email** bot without needing a matching member or sender
   authentication pass. It shares the normal 30-messages-per-hour room
   limit, so request it before forwarding a busy inbox.
3. Return to Gmail, enable forwarding or create a forwarding filter, and
   save. Send a fresh message to the inbox and confirm it appears in the
   room. Gmail's [forwarding instructions](https://support.google.com/mail/answer/10957)
   cover this confirmation step. Gmail does not automatically forward spam.

For a Google Group, add the room address as an external member with
**Each email** delivery. Workspace and group policy must allow external
members. Moderation, spam filtering, or digest delivery can delay or
prevent delivery before Smartfire receives anything.

Forwarding preserves the original `From` and often the inbox or group in
`To`. The relay must preserve the room's final recipient as described
above. Forwarded mail from an active room member posts as that member when
the relay reports `dkim=pass` with `header.d` matching the `From` domain
exactly. A valid signature from the author's domain can survive forwarding,
even when the room appears only in a delivery header. If forwarding breaks
the signature, or the author's domain does not DKIM-sign, mail posts as the
**Email** bot naming the sender. SPF and DMARC passes do not establish member
identity, including for Gmail forwarding and Google Groups delivery.

Mail over 30 messages per hour per room is dropped silently. The relay
and Gmail see success, so forwarding a busy inbox can lose mail.

Mail uses normal room unread state and notifications, including
each member's involvement, mute, and do-not-disturb settings. Choose
**Everything** to be notified of every incoming message. People with inbox
access must also join the room. Forwarding does not synchronize access.
Anyone in the room can read forwarded mail and the confirmation message.

## Sender verification

`From` is trivially spoofable, so a member address alone never decides
who a message posts as. Mail posts as the matched active room member only
when the topmost matching relay header verifies DKIM. The app selects the
`Authentication-Results` header ([RFC 8601](https://www.rfc-editor.org/rfc/rfc8601.html))
whose authserv-id matches `INBOUND_EMAIL_AUTHSERV_ID` and appears first.
That header must report `dkim=pass` with exactly one `header.d` property
matching the `From` domain exactly. Member attribution also requires exactly
one `From` field containing exactly one mailbox. Fields with whitespace
before the colon, such as `From :` and `From\t:`, count as `From` fields.
Result clauses and properties ignore nested comments and treat quoted
strings as opaque, so neither can supply a hidden
`header.d` property or clause separator.
The comparison ignores domain case but does not accept a parent or
subdomain match. This rule applies to both direct and forwarded mail.
`INBOUND_EMAIL_AUTHSERV_ID` must name a relay that escapes comment text in
`Authentication-Results` or never copies the envelope sender's local part into comments.
`To`, `Cc`, and `Bcc` do not prove identity because the sender writes them.
An unconfigured id, no matching header, a failed or missing DKIM result,
or a pass for another domain means the mail posts as the **Email** bot.
Members whose domains do not DKIM-sign also post as the bot.

DKIM verifies a signature by the author's domain. SPF authenticates the
envelope sender, which a forwarder can rewrite onto its own domain even
when `From` is spoofed. DMARC can pass through SPF alignment alone.
Neither `spf=pass` nor `dmarc=pass` counts as proof for member attribution.

Trusted headers: only the relay's own `Authentication-Results` field
is trusted. Fields from any other authserv-id, including ones the
sender forged, are ignored, as is every other sender header
(`Received-SPF`, `DKIM-Signature`, `X-Spam-*`, `X-Mailgun-*`). The
relay **must stamp its own `Authentication-Results` header** at
receive time (prepended, so it is the topmost field), and should
still strip any incoming `Authentication-Results` headers first: a
forged field carrying the relay's own id would otherwise verify on a
message the relay did not stamp. If the relay stamps no results for a
message, member mail from that message posts as the bot. If your relay
or adapter supplies verdicts only in proprietary headers or POST fields,
leave `INBOUND_EMAIL_AUTHSERV_ID` unset.
Everything posts as the bot, and there is no id for a forger to guess.
This limitation does not apply to Cloudflare Email Routing, which
supplies the standard header linked in [Ingress setup](#ingress-setup-relay-provider).
A standard header still needs `dkim=pass` and the matching `header.d`
for Smartfire to attribute mail to a member.
