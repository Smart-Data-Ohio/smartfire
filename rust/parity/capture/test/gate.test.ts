import { test } from 'node:test'
import assert from 'node:assert/strict'
import fs from 'node:fs'
import path from 'node:path'
import { PNG } from 'pngjs'
import { PARITY_DIR, SEED_DIR, VIEWPORTS } from '../config.ts'
import { compareJob } from '../compare.ts'
import { Allowlist } from '../allowlist.ts'
import { run } from '../run.ts'
import { seedFingerprint, validateSeed } from '../seed_validation.ts'
import { EventEmitter } from 'node:events'
import { NetworkLog } from '../network.ts'
import http from 'node:http'
import { gzipSync } from 'node:zlib'
import { startProxy } from '../proxy.ts'

fs.mkdirSync(path.join(PARITY_DIR, 'out'), { recursive: true })

const job = { state: { id: 'gate/probe', path: '/', seed: 'default', steps: [] }, cell: { engine: 'chromium' as const, viewport: VIEWPORTS.desktop, scheme: 'light' as const } }
test('recompare cannot clear a previously recorded transient pixel failure', () => {
  const out = fs.mkdtempSync(path.join(PARITY_DIR, 'out/ws19-flake-'))
  try {
    const attempts = [{ attempt: 1, status: 'fail', differentPixels: 1 }, { attempt: 2, status: 'pass', differentPixels: 0 }]
    for (const side of ['expected', 'actual']) {
      const dir = path.join(out, side, job.state.id); fs.mkdirSync(dir, { recursive: true })
      const base = path.join(dir, 'chromium-desktop-light')
      fs.writeFileSync(base + '.png', PNG.sync.write(new PNG({ width: 1, height: 1 })))
      fs.writeFileSync(base + '.json', JSON.stringify({ kind: 'page', status: 200, flakyAttempts: attempts }))
      for (const suffix of ['server.norm.html', 'live.norm.html', 'aria.yml', 'network.txt', 'cable.txt']) fs.writeFileSync(`${base}.${suffix}`, 'equal')
    }
    const result = compareJob(job, out, 'expected', 'actual', new Allowlist([]))
    assert.equal(result.status, 'fail'); assert.equal(result.flaky, true)
    assert.equal(result.attempts?.[0].differentPixels, 1)
  } finally { fs.rmSync(out, { recursive: true, force: true }) }
})
test('presence-sensitive cells reject shared-server captures without a reset hook', async () => {
  const out = fs.mkdtempSync(path.join(PARITY_DIR, 'out/ws19-presence-'))
  try {
    await assert.rejects(run({ states: [{ ...job.state, isolated: true }], targets: [], filter: { matrix: 'lean' }, outDir: out, time: '2026-03-02T16:00:00Z', workers: 1, timeoutMs: 8000, quiet: true }), /presence-sensitive states require fresh servers/)
  } finally { fs.rmSync(out, { recursive: true, force: true }) }
})
test('changed seed bytes and clock invalidate a validation receipt', () => {
  const root = fs.mkdtempSync(path.join(PARITY_DIR, 'out/ws19-receipt-'))
  const previous = process.env.PARITY_SEED_VALIDATION_FILE
  try {
    fs.cpSync(path.join(SEED_DIR, 'default'), path.join(root, 'default'), { recursive: true })
    const time = '2026-03-02T16:00:00.000Z'
    const receipt = path.join(root, 'receipt.json')
    fs.writeFileSync(receipt, JSON.stringify({ seed: 'default', time, fingerprint: seedFingerprint('default', root) }))
    process.env.PARITY_SEED_VALIDATION_FILE = receipt
    assert.throws(() => validateSeed('default', root, '2026-03-03T16:00:00.000Z'), /receipt does not match/)
    fs.appendFileSync(path.join(root, 'default/db/production.sqlite3'), 'wrong seed')
    assert.throws(() => validateSeed('default', root, time), /receipt does not match/)
  } finally {
    if (previous === undefined) delete process.env.PARITY_SEED_VALIDATION_FILE; else process.env.PARITY_SEED_VALIDATION_FILE = previous
    fs.rmSync(root, { recursive: true, force: true })
  }
})
test('a digest-named asset compares its actual response bytes', async () => {
  const describe = async (body: string) => {
    const page = new EventEmitter()
    const log = new NetworkLog(page as any, 'http://localhost:3999', {})
    page.emit('response', {
      url: () => 'http://localhost:3999/assets/app-12345678.css', status: () => 200,
      request: () => ({ url: () => 'http://localhost:3999/assets/app-12345678.css', method: () => 'GET', resourceType: () => 'stylesheet', isNavigationRequest: () => false }),
      allHeaders: async () => ({ 'content-type': 'text/css' }), headersArray: async () => [{ name: 'content-type', value: 'text/css' }], body: async () => Buffer.from(body),
    })
    return log.text()
  }
  assert.notEqual(await describe('body {}'), await describe('body {} :root { --unused: wrong; }'))
})
test('the cell proxy retains actual compressed asset bytes and exact memory-cache reuse', async () => {
  let content = 'body {}'
  const server = http.createServer((_, res) => { res.writeHead(200, { 'content-encoding': 'gzip' }); res.end(gzipSync(content)) })
  await new Promise<void>(resolve => server.listen(49891, '127.0.0.1', resolve))
  const proxy = await startProxy('http://127.0.0.1:49891')
  const asset = 'http://localhost:3999/assets/app-12345678.css'
  const get = () => new Promise<void>((resolve, reject) => {
    const url = new URL(proxy.server)
    http.get({ hostname: url.hostname, port: url.port, path: asset, agent: false }, response => { response.resume(); response.on('end', resolve) }).on('error', reject)
  })
  try {
    await get(); assert.equal((await proxy.assetBody(asset)).toString(), content)
    assert.equal((await proxy.assetBody(asset)).toString(), content) // no second server request
    content += ' :root { --unused: wrong; }'
    await get(); assert.equal((await proxy.assetBody(asset)).toString(), content)
    await assert.rejects(proxy.assetBody(asset.replace('app-', 'missing-')), /no captured upstream response/)
  } finally {
    await proxy.close()
    await new Promise<void>(resolve => server.close(() => resolve()))
  }
})
