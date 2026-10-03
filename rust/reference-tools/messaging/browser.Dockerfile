# Pinned app, process config, and opt-in ActiveJob::TestHelper boundary only.
ARG BASE_IMAGE=triage-reference-d7c7de92
FROM ${BASE_IMAGE}
USER root
COPY --chown=1000:1000 parity/docker/resque-pool.yml /rails/config/resque-pool.yml
COPY --chown=1000:1000 parity/docker/Procfile /rails/Procfile
COPY --chown=1000:1000 reference-tools/messaging/browser-test-jobs.rb /rails/config/initializers/ws8bm_browser_test_jobs.rb
COPY --chown=1000:1000 reference-tools/messaging/browser-drive.rb /rails/config/initializers/ws8bm_browser_drive.rb
USER 1000:1000
