// End-to-end fault injection against the real Rails image and pinned browser/CLI gate.
// Run: PARITY_NAMESPACE=ws19 PARITY_OWNER=ws19 node capture/test/injections/suite.ts before|after
import fs from 'node:fs'
import path from 'node:path'
import http from 'node:http'
import { spawn } from 'node:child_process'
import assert from 'node:assert/strict'
import { PARITY_DIR } from '../../config.ts'
const phase = process.argv[2]
assert.ok(['before', 'after'].includes(phase))
const orderingOnly = process.argv.includes('--ordering-only')
const out = path.join(PARITY_DIR, 'out/ws19-review-fixes', orderingOnly ? `ordering-${phase}` : phase)
fs.mkdirSync(out, { recursive: true })
const reference = path.join(PARITY_DIR, 'bin/reference')
const compare = path.join(PARITY_DIR, 'bin/compare')
const time = '2026-03-02T16:00:00Z'
const mutations: Record<string, (h: http.OutgoingHttpHeaders) => void> = {
  frame_options: h => { h['x-frame-options'] = 'DENY' },
  csp_policy: h => { h['content-security-policy-report-only'] = "default-src 'self'; object-src https://wrong.invalid" },
  cookie_value: h => { h['set-cookie'] = ['oracle=wrong; Path=/; SameSite=Lax; Max-Age=3600; Expires=Mon, 02 Mar 2026 17:00:00 GMT'] },
  cookie_path: h => { h['set-cookie'] = ['oracle=expected; Path=/restricted; SameSite=Lax; Max-Age=3600; Expires=Mon, 02 Mar 2026 17:00:00 GMT'] },
  cookie_samesite: h => { h['set-cookie'] = ['oracle=expected; Path=/; SameSite=Strict; Max-Age=3600; Expires=Mon, 02 Mar 2026 17:00:00 GMT'] },
  cookie_maxage: h => { h['set-cookie'] = ['oracle=expected; Path=/; SameSite=Lax; Max-Age=1; Expires=Mon, 02 Mar 2026 17:00:00 GMT'] },
  cookie_expires: h => { h['set-cookie'] = ['oracle=expected; Path=/; SameSite=Lax; Max-Age=3600; Expires=Mon, 02 Mar 2026 16:00:01 GMT'] },
  session_path: h => { h['set-cookie'] = ['_campfire_session=random; Path=/restricted; SameSite=Lax; Max-Age=3600'] },
  cookie_order: h => { h['set-cookie'] = ['oracle=two; Path=/; SameSite=Lax', 'oracle=one; Path=/; SameSite=Lax'] },
  duplicate_path: h => { h['set-cookie'] = ['oracle=expected; Path=/; Path=/restricted'] },
}
const states: any[] = [{ id: 'injection/control', path: '/session/new', steps: [] }]
// A Map lookup never reaches Object.prototype members for header-supplied names.
const mutationByName = new Map(Object.entries(mutations))
for (const kind of ['page', 'fragment']) for (const mutation of Object.keys(mutations)) {
  const name = `${kind}_${mutation}`
  states.push({ id: `injection/${name}`, path: '/session/new', kind, steps: [], headers: { 'x-oracle-injection': name }, request: { headers: { 'x-oracle-injection': name } } })
}
states.push({ id: 'injection/asset_body', path: '/session/new', steps: [], headers: { 'x-oracle-injection': 'asset_body' } })
const inventory = path.join(out, 'inventory.json')
fs.writeFileSync(inventory, JSON.stringify({ states }))
async function command(exe: string, args: string[], log: string, env = process.env) {
  const stream = fs.createWriteStream(path.join(out, log))
  const child = spawn(exe, args, { env, stdio: ['ignore', 'pipe', 'pipe'] })
  child.stdout.pipe(stream, { end: false }); child.stderr.pipe(stream, { end: false })
  const code = await new Promise<number>(resolve => child.on('exit', code => resolve(code ?? 1)))
  stream.end()
  return code
}
const servers: http.Server[] = []
try {
  assert.equal(await command(reference, ['up', '--seed', 'default', '--port', '49801', '--time', time, '--freeze'], 'reference.log'), 0)
  for (const [port, actual] of [[49881, false], [49882, true]] as const) {
    const server = http.createServer((req, res) => {
      const name = String(req.headers['x-oracle-injection'] ?? '')
      const upstream = http.request({ hostname: '127.0.0.1', port: 49801, path: req.url, method: req.method, headers: { ...req.headers, 'accept-encoding': 'identity' } }, response => {
        const chunks: Buffer[] = []
        response.on('data', b => chunks.push(b)).on('end', () => {
          let body = Buffer.concat(chunks)
          const h: http.OutgoingHttpHeaders = { ...response.headers }
          delete h['content-length']; delete h['transfer-encoding']; delete h.connection
          if (req.url === '/session/new') {
            h['x-frame-options'] = 'SAMEORIGIN'
            h['content-security-policy-report-only'] = "default-src 'self'; object-src 'none'"
            h['set-cookie'] = [name.endsWith('session_path') ? '_campfire_session=random; Path=/; SameSite=Lax; Max-Age=3600' : 'oracle=expected; Path=/; SameSite=Lax; Max-Age=3600; Expires=Mon, 02 Mar 2026 17:00:00 GMT']
            if (name.endsWith('cookie_order')) h['set-cookie'] = ['oracle=one; Path=/; SameSite=Lax', 'oracle=two; Path=/; SameSite=Lax']
            if (name.endsWith('duplicate_path')) h['set-cookie'] = ['oracle=expected; Path=/restricted; Path=/']
            const mutate = mutationByName.get(name.replace(/^(page|fragment)_/, ''))
            if (actual && typeof mutate === 'function') mutate(h)
          }
          if (actual && name === 'asset_body' && /\.css(?:\?|$)/.test(req.url!)) body = Buffer.concat([body, Buffer.from('\nhtml { --oracle-sentinel: wrong; }')])
          res.writeHead(response.statusCode!, h); res.end(body)
        })
      })
      upstream.on('error', e => { res.writeHead(502); res.end(String(e)) }); req.pipe(upstream)
    })
    await new Promise<void>(resolve => server.listen(port, '127.0.0.1', resolve)); servers.push(server)
  }
  const common = ['--expected', 'http://127.0.0.1:49881', '--actual', 'http://127.0.0.1:49882', '--engines', 'chromium', '--viewports', 'desktop', '--schemes', 'light', '--time', time, '--no-allowlist', '--workers', '2']
  const selection = orderingOnly ? ['--only', 'injection/control,injection/*cookie_order,injection/*duplicate_path'] : []
  const code = await command(compare, [...common, '--inventory', inventory, ...selection, '--out', path.join(out, 'differences')], 'differences.log')
  const report = JSON.parse(fs.readFileSync(path.join(out, 'differences/report.json'), 'utf8'))
  assert.equal(report.results.length, orderingOnly ? 5 : states.length)
  for (const result of report.results) {
    const expected = result.state.endsWith('/control') || phase === 'before' ? 'pass' : 'fail'
    console.log(`${result.state}: ${result.status} (expected ${expected})`)
    assert.equal(result.status, expected)
  }
  assert.equal(code, phase === 'before' ? 0 : 1)
  if (!orderingOnly) {
    const pixelCode = await command(compare, [...common, '--inventory', inventory, '--only', 'injection/control', '--out', path.join(out, 'flaky')], 'flaky.log', { ...process.env, NODE_OPTIONS: `--import=${path.join(PARITY_DIR, 'capture/test/injections/first_pixel.ts')}` })
    const flaky = JSON.parse(fs.readFileSync(path.join(out, 'flaky/report.json'), 'utf8'))
    assert.equal(flaky.info.flaky, 1)
    assert.equal(pixelCode, phase === 'before' ? 0 : 1)
    console.log(`first-capture pixel: ${flaky.info.flaky} flaky, exit ${pixelCode}`)
    // Corrupt a disposable snapshot, preserving the real seed.
    const seedDir = path.join(out, 'bad-seeds')
    fs.cpSync(path.join(PARITY_DIR, '.seed/default'), path.join(seedDir, 'default'), { recursive: true })
    assert.equal(await command('python3', ['-c', 'import sqlite3,sys; c=sqlite3.connect(sys.argv[1]); c.execute("delete from message_pins"); c.commit()', path.join(seedDir, 'default/db/production.sqlite3')], 'delete-pins.log'), 0)
    const badCode = await command(compare, ['--self-parity', '--only', 'channels/pins', '--seed', 'default', '--seed-dir', seedDir, '--ports', '49401,49402', '--out', path.join(out, 'bad-seed')], 'bad-seed.log', { ...process.env, PARITY_SEED_DIR: seedDir })
    assert.equal(badCode, phase === 'before' ? 0 : 1)
    console.log(`empty message_pins seed: exit ${badCode}`)
    // Other captures on a shared Rails server establish David's presence lease.
    assert.equal(await command(reference, ['exec', '--port', '49801', '--', 'bin/rails', 'runner', 'u=User.find_by!(email_address: "david@37signals.com"); WorkspacePresenceLease.establish(user: u, session: u.sessions.first!)'], 'presence-inject.log'), 0)
    const presenceCode = await command(compare, [...common.map(value => value.replace('49881', '49801').replace('49882', '49801')), '--only', 'account/self', '--out', path.join(out, 'presence')], 'presence.log')
    assert.equal(presenceCode, phase === 'before' ? 0 : 1)
    console.log(`shared presence activity without isolation: exit ${presenceCode}`)
  }
  console.log(`${phase}: all injection expectations satisfied`)
} finally {
  await Promise.all(servers.map(server => new Promise<void>(resolve => server.close(() => resolve()))))
  await command(reference, ['down', '--port', '49801'], 'cleanup.log')
}
