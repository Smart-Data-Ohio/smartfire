# Email to room

Any chat room (direct and board rooms excluded) can have a secret forward-to address.
Mail sent there posts in the room: from the matched member when the
sender's address belongs to an active room member, otherwise from the
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
nothing errors.

## What arrives

- The subject becomes the message's first line, then the body. Mail from
  a non-member is prefixed with `From <sender>`.
- Plain text is preferred. HTML-only mail is sanitized (script and style
  elements removed, the rest run through the safe-list sanitizer) and
  stripped to plain text; the stored message is never raw HTML.
- The first attachment within the 10 MB limit lands on the message
  (messages carry one file); every file is named in the body, and files
  over the limit are named with the reason instead of arriving silently
  missing.
- Bodies are capped at the normal message length. Empty mail with no
  attachment posts nothing.
- Unknown tokens post nothing and bounce nothing, so the address reveals
  nothing to probers.

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
   - **Mailgun:** a route whose action forwards to the relay URL with
     basic-auth credentials in the URL
     (`https://actionmailbox:<password>@<app host>/rails/action_mailbox/relay/inbound_emails`).
   - **SendGrid:** Inbound Parse pointed at the relay URL with basic auth
     enabled, username `actionmailbox`.
   - **Self-hosted Postfix/Exim/Qmail:** pipe inbound mail with the
     matching `bin/rails action_mailbox:ingress:<server>` command, as the
     [relay ingress docs](https://guides.rubyonrails.org/action_mailbox_basics.html#relay-ingress)
     describe.
4. Send a test message to a room's address and confirm it lands in the
   room; rotate the room's address afterwards if the test address was
   shared.

Inbound mail is deduplicated by message id, and processing runs through
the normal job backend, so a failed run retries like any other job.
