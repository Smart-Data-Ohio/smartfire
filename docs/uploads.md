# Upload size policy

Workspace administrators set the maximum file size under **Workspace > Uploads**. The
default is 100 MB, where the UI uses 1 MB = 1,048,576 bytes. The policy is stored as
`upload_limit_bytes` in the existing `accounts.settings` JSON. Missing or invalid stored
values use the default. No schema migration is needed.

The SPA boot account and the workspace API publish `uploadLimitBytes`. The composer rejects
oversized files before adding them to its tray. Every SPA direct upload checks the limit
before hashing, creating a blob or sending bytes. The server also rejects oversized blob
creation with a readable 422, through both `/api/v1/uploads` and the retained Active Storage
endpoint.

## Transport limits

The deployment in `deploy/gcp` is ONCE on a Compute Engine VM. Uploads use signed
`/rails/active_storage/disk/...` URLs on the app, not GCS signed URLs. The raw PUT bypasses
the kit's 16 MiB buffered body parser. The adapter spools chunks to a temporary file, and
the disk service streams that file into storage while verifying its checksum. The browser
hashes in 2 MiB chunks and sends the original File through XHR.

The front server defaults `MAX_REQUEST_BODY` to 0, meaning no transport size ceiling.
The Dockerfile sets the read and write timeouts to 300 seconds. With those defaults,
the application ceiling is the configured policy, initially 104,857,600 bytes. The regression
test `upload_size_100_mb_streams_through_the_real_front_server_to_disk` sends that many
bytes over HTTP/1.1 through the front server and checks the stored content. No infrastructure
change was needed for this VM path. Production traffic was not tested by this slice.

If an operator sets `MAX_REQUEST_BODY`, `THRUSTER_MAX_REQUEST_BODY`, or a reverse proxy limit
below the policy, that smaller ceiling wins. Allow sufficient temporary and final disk space,
including filesystem quotas.

Cloud Run with an HTTP/1 backend has a [32 MiB request ceiling](https://docs.cloud.google.com/run/quotas).
This upload path cannot support 100 MB on that configuration. Such a deployment would need
end-to-end HTTP/2 with the app's H2C listener enabled, or a separate storage upload route.
