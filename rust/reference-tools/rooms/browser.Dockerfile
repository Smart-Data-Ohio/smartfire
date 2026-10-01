# The pinned Rails application remains unchanged. Reuse the parity harness's process config.
ARG BASE_IMAGE=ws8br-reference-d7c7de92
FROM ${BASE_IMAGE}
USER root
COPY --chown=1000:1000 resque-pool.yml /rails/config/resque-pool.yml
COPY --chown=1000:1000 Procfile /rails/Procfile
USER 1000:1000
