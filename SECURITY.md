# Security

Report vulnerabilities through [GitHub security
advisories](https://github.com/Smart-Data-Ohio/smartfire/security/advisories/new).

## Trust model

Smartfire is self-hosted and single-tenant, so the administrator is the server operator,
with shell, network, and database access already. Anything requiring the administrator
role grants nothing they do not already have, and is not a vulnerability.

We do want reports of anything a **non-administrator** can reach, and of any credential or
network path held by the Smartfire process but not by the operator's own shell.

## Intentional behavior

Every webhook POST, legacy bot or agent delivery, resolves through
`RestrictedHTTP::PrivateNetworkGuard` and pins the connection to the
resolved public address, like link unfurling: loopback and private
destinations are refused instead of posted to. Operators who need bots
on internal services must expose them at a public address the guard
accepts.
