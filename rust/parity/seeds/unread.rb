# Readiness connects PresenceChannel, which clears the current room's unread flag.
# Show unread badges from a non-room page, so first sidebar responses are deterministic too.
based_on "default"
at NOW
unread! :watercooler, :david, message(:bot_in_watercooler).created_at
unread! :david_and_kevin, :david, message(:direct_unread).created_at
unread! :group_direct, :david, message(:group_direct_first).created_at
