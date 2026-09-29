# Parity-only (baked into the campfire-reference image by parity/docker/Dockerfile, not part of
# the reference app): accept migrations dated after the frozen seed clock.
#
# Rails refuses to load a migration whose timestamp is more than a day ahead of "now". Instances
# run with the clock at the seed's instant (2026-03-02, see parity/seeds/lib/seed.rb), before most
# of the reference's migrations, so `bin/rails db:prepare` in bin/start-app would abort at boot.
# The database is already migrated when the clock is faked; this only skips the date check.
Rails.application.config.active_record.validate_migration_timestamps = false
