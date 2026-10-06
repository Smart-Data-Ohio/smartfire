# The remaining system declarations start with fixtures :all, not the visual
# default seed's extra people, rooms, messages, status, integrations and threads.
load_fixtures

# Each browser signs in through the original test-session GET, which verifies
# real credentials and creates its own verified session and device cookie.
