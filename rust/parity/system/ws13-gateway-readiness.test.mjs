import assert from 'node:assert/strict';
import {test} from 'node:test';
import {mkdtemp, readFile, writeFile, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {publishGatewayPort} from './ws13-gateway-readiness.mjs';

test('a partially written gateway port cannot appear ready to Rust', async t => {
  const temp = await mkdtemp(join(tmpdir(), 'ws13-ready-'));
  t.after(() => rm(temp, {recursive:true, force:true}));
  const path = join(temp, 'gateway-port');
  const opened = Promise.withResolvers();
  const finish = Promise.withResolvers();
  const publishing = publishGatewayPort(path, 52300, async (file, value) => {
    await writeFile(file, '');
    opened.resolve();
    await finish.promise;
    await writeFile(file, value);
  });
  await opened.promise;
  try {
    await assert.rejects(readFile(path, 'utf8'), {code:'ENOENT'});
  } finally {
    finish.resolve();
    await publishing;
  }
  assert.equal(await readFile(path, 'utf8'), '52300');
});
