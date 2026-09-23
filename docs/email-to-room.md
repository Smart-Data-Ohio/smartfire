# Email to room

Any chat room (direct and board rooms excluded) can have a secret forward-to address.
Mail sent there posts in the room: from the matched member when the
sender's address belongs to an active room member *and* the relay's
own `Authentication-Results` header reports SPF, DKIM or DMARC
passing for the `From` domain (see
[Sender verification](#sender-verification)), otherwise from the
workspace **Email** bot with the sender named in the body.

## The address

Each room's address looks like
`room-<random-token>@<configured domain>`. Anyone who can administer the
room (an administrator, or its creator) creates or rotates it from the
room's settings page; rotating retires the old address immediately.
Direct and board rooms never have an address, and deleted rooms receive
nothing.

While `INBOUND_EMAIL_DOMAIN` is unset, inbound email is disabled cleanly:
the mailbox drops everything, the room settings explain what to set, and
nothing errors. Member verification additionally needs
`INBOUND_EMAIL_AUTHSERV_ID` (see
[Sender verification](#sender-verification)); while it is unset,
everything that arrives posts as the **Email** bot.

## What arrives

- The subject becomes the message's first line, then the body. Mail from
  a non-member is prefixed with `From <sender>`.
- Plain text is preferred. HTML-only mail is sanitized (script and style
  elements removed, the rest run through the safe-list sanitizer) and
  stripped to plain text; the stored message is never raw HTML.
- The first attachment within the 10 MB limit lands on the message
  (messages carry one file); every file is named in the body, and files
  over the limit are named with the reason instead of arriving silently
  missing. The size is estimated from the encoded MIME part before
  decoding, so a huge part is rejected without paying for the decode.
- Only images, PDFs, text files, and office documents are attached;
  other types are named with `file type not allowed` and never land on
  the message.
- Bodies are capped at the normal message length. Empty mail with no
  attachment posts nothing.
- Each room accepts at most 30 emailed messages per hour (hour bucket
  in `Rails.cache`, like the agent API throttle); over-limit mail is
  dropped silently.
- Unknown tokens post nothing and bounce nothing, so the address reveals
  nothing to probers. Mail to a non-room address (no `room-` token at
  all) is marked bounced instead of raising a routing error — with no
  reply sent, since the app has no outbound mail and answering
  misaddressed mail would backscatter.

## Ingress setup (relay provider)

Production accepts inbound mail through the Action Mailbox relay ingress
(`config.action_mailbox.ingress = :relay` in
`config/environments/production.rb`). The relay endpoint is
`<app root URL>/rails/action_mailbox/relay/inbound_emails`, authenticated
with HTTP basic auth as user `actionmailbox`.

1. Generate a strong password and give it to the app as the
   `RAILS_INBOUND_EMAIL_PASSWORD` environment variable (or
   `action_mailbox.ingress_password` in encrypted credentials). Serve the
   app over HTTPS: basic auth over plain HTTP leaks the password.
2. Point the inbound domain's MX records at the relay provider (for
   example Postmark's inbound stream, Mailgun Routes, or SendGrid's
   Inbound Parse) following that provider's domain-verification steps.
   This is DNS/org configuration outside this repo; nothing here changes
   it.
3. Configure the provider to forward each message's raw RFC 822 bytes to
   the relay endpoint with basic auth:
   - **Postmark:** an inbound rule whose webhook posts the raw message
     (enable "Post the full raw content") to the relay URL with the
     basic-auth username `actionmailbox` and the ingress password.
     Postmark stamps SpamAssassin verdict headers (`X-Spam-Status`,
     `X-Spam-Score`, `X-Spam-Tests`), not an `Authentication-Results`
     header ([inbound webhook docs](https://postmarkapp.com/developer/webhooks/inbound-webhook)).
   - **Mailgun:** a route whose action forwards to the relay URL with
     basic-auth credentials in the URL
     (`https://actionmailbox:<password>@<app host>/rails/action_mailbox/relay/inbound_emails`).
     Mailgun stamps `X-Mailgun-Spf` and `X-Mailgun-Dkim-Check-Result`
     (plus `X-Mailgun-Sflag`/`X-Mailgun-Sscore` when spam filtering
     marks headers), not an `Authentication-Results` header
     ([spam filter docs](https://documentation.mailgun.com/docs/mailgun/user-manual/receive-forward-store/spam-filter/)).
   - **SendGrid:** Inbound Parse pointed at the relay URL with basic auth
     enabled, username `actionmailbox`. SendGrid reports SPF/DKIM
     results as separate `SPF` and `dkim` Inbound Parse POST fields,
     not as an `Authentication-Results` header on the raw message
     ([Inbound Parse docs](https://www.twilio.com/docs/sendgrid/for-developers/parsing-email/setting-up-the-inbound-parse-webhook)).
   - **Self-hosted Postfix/Exim/Qmail:** pipe inbound mail with the
     matching `bin/rails action_mailbox:ingress:<server>` command, as the
     [relay ingress docs](https://guides.rubyonrails.org/action_mailbox_basics.html#relay-ingress)
     describe. A DKIM/DMARC milter (OpenDKIM, OpenDMARC, Rspamd) on
     your MTA stamps `Authentication-Results` with your hostname as
     the authserv-id.

   Whichever relay you use, it must stamp its own
   `Authentication-Results` header for member verification to work:
   the app trusts only the topmost such header whose authserv-id
   matches `INBOUND_EMAIL_AUTHSERV_ID`, and reads no other sender
   verdict (`Received-SPF`, `DKIM-Signature`, `X-Spam-*`,
   `X-Mailgun-*`, SendGrid's POST fields). The SaaS relays above
   report their verdicts only in those proprietary headers and
   fields, so their mail always posts as the **Email** bot. Set the
   authserv id from a received message's headers — the identifier
   before the first `;` in your relay's field — or leave it unset.
4. Send a test message to a room's address and confirm it lands in the
   room; rotate the room's address afterwards if the test address was
   shared.

Inbound mail is deduplicated by message id, and processing runs through
the normal job backend, so a failed run retries like any other job.

## Sender verification

`From` is trivially spoofable, so a member address alone never decides
who a message posts as. Mail posts as the matched member only when the
topmost `Authentication-Results` header ([RFC 8601](https://www.rfc-editor.org/rfc/rfc8601.html))
whose authserv-id matches `INBOUND_EMAIL_AUTHSERV_ID` reports a pass
aligned with the `From` domain. Anything else — the id unconfigured,
no header matching it, a failure, or a pass for another domain —
posts as the **Email** bot with the sender shown.

A pass counts only with its own method's domain property matching the
`From` domain exactly:

- `dkim=pass` with `header.d` matching,
- `dmarc=pass` with `header.from` matching,
- `spf=pass` with `smtp.mailfrom` matching.

An `spf=pass` on `smtp.helo` alone never verifies: the HELO name says
which server connected, not who the message is from.

Trusted headers: only the relay's own `Authentication-Results` field
is trusted. Fields from any other authserv-id — including ones the
sender forged — are ignored, as is every other sender header
(`Received-SPF`, `DKIM-Signature`, `X-Spam-*`, `X-Mailgun-*`). The
relay **must stamp its own `Authentication-Results` header** at
receive time (prepended, so it is the topmost field), and should
still strip any incoming `Authentication-Results` headers first: a
forged field carrying the relay's own id would otherwise verify on a
message the relay did not stamp. If the relay stamps no results for a
message, member mail from that message posts as the bot — degraded,
but safe. While your relay stamps no `Authentication-Results` at all
(the SaaS relays in [Ingress setup](#ingress-setup-relay-provider)
report their verdicts only in proprietary headers or POST fields),
leave `INBOUND_EMAIL_AUTHSERV_ID` unset: everything posts as the bot,
and there is no id for a forger to guess.
