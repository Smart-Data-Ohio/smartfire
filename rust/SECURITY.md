# Security

Report vulnerabilities through our [HackerOne program](https://hackerone.com/basecamp) or at
https://github.com/basecamp/once-campfire-rust/security. Note that Campfire is in scope for our
security program, but is not bounty eligible.

## Trust model

Campfire is self-hosted and single-tenant, so the administrator is the server operator,
with shell, network, and database access already. Anything requiring the administrator
role grants nothing they do not already have, and is not a vulnerability.

We do want reports of anything a **non-administrator** can reach, and of any credential or
network path held by the Campfire process but not by the operator's own shell. That includes
anything that lets a member exhaust the server (CPU, memory, disk, connections) out of
proportion to what they send.

## Intentional behavior

Bot webhook URLs are unrestricted: an administrator can point one at any address,
including internal ones, because operators legitimately wire bots to their own services.
Link unfurling and Web Push are different because any member can trigger them, so their
destinations are checked against private and reserved networks (`integrations/net/guard.rs`)
and the checked address is the one connected to. The difference is who picks the destination.
