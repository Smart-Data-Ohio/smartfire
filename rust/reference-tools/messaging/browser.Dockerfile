# Pinned app, process config, and opt-in ActiveJob::TestHelper boundary only.
ARG BASE_IMAGE=campfire-reference
FROM ${BASE_IMAGE}
USER root
# Match parity/bin/ci-seed: pinned behavior with the current required schema.
COPY --from=current_schema --chown=1000:1000 schema.rb /rails/db/schema.rb
COPY --from=current_schema --chown=1000:1000 migrate /rails/db/migrate
COPY --chown=1000:1000 parity/docker/resque-pool.yml /rails/config/resque-pool.yml
COPY --chown=1000:1000 parity/docker/Procfile /rails/Procfile
COPY --chown=1000:1000 reference-tools/messaging/browser-test-jobs.rb /rails/config/initializers/ws8bm_browser_test_jobs.rb
COPY --chown=1000:1000 reference-tools/messaging/browser-drive.rb /rails/config/initializers/ws8bm_browser_drive.rb
# The pinned test routes require this file before engine routes can finish.
# The production image omits test/, so restore the original test fixture.
COPY --chown=1000:1000 reference-tools/messaging/browser-test-session-controller.rb /rails/test/support/test_session_controller.rb
COPY --chown=1000:1000 reference-tools/messaging/browser-client-message.js /rails/app/javascript/models/client_message.js
USER 1000:1000
# Approved Rails drift #231: build the real changed client, never a page mask.
RUN SECRET_KEY_BASE_DUMMY=1 RAILS_ENV=production bundle exec rails assets:clobber assets:precompile
