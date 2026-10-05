import {writeFile, rename} from 'node:fs/promises';

// Rust treats existence as readiness. Publish only after all port bytes are written.
export async function publishGatewayPort(path, port, write = writeFile) {
  const pending = path + '.pending';
  await write(pending, String(port));
  await rename(pending, path);
}
