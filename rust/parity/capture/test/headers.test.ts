import { test } from 'node:test'
import assert from 'node:assert/strict'
import { responseHeaders } from '../headers.ts'
import { createHash } from 'node:crypto'
const options = { seedTime: Date.parse('2026-03-02T16:00:00Z'), frozenServerClock: true }
const describe = (name: string, value: string) => responseHeaders([{ name, value }], options)
test('all application header values and header presence matter', () => {
  for (const name of ['X-Frame-Options', 'Permissions-Policy', 'Content-Security-Policy', 'X-App-Feature']) {
    assert.notEqual(describe(name, 'expected'), describe(name, 'wrong'))
    assert.notEqual(describe(name, 'expected'), responseHeaders([], options))
  }
})
test('CSP normalizes nonce bytes and preserves every directive', () => {
  assert.equal(describe('content-security-policy', "script-src 'nonce-a'; object-src 'none'"), describe('content-security-policy', "script-src 'nonce-b'; object-src 'none'"))
  assert.notEqual(describe('content-security-policy', "script-src 'nonce-a'; object-src 'none'"), describe('content-security-policy', "script-src 'nonce-b'; object-src 'self'"))
})
test('only explicit volatile header fields are normalized', () => {
  for (const name of ['x-request-id', 'x-csrf-token']) assert.equal(describe(name, 'a'), describe(name, 'b'))
  assert.equal(describe('date', 'Mon, 02 Mar 2026 16:00:00 GMT'), describe('date', 'Tue, 03 Mar 2026 16:00:00 GMT'))
  assert.notEqual(describe('expires', 'Mon, 02 Mar 2026 16:00:00 GMT'), describe('expires', 'Tue, 03 Mar 2026 16:00:00 GMT'))
  assert.notEqual(describe('x-csrf-token', ''), describe('x-csrf-token', 'value'))
})
test('cookies preserve values containing equals, scope, lifetime and flags', () => {
  const original = 'preference=a=b; Path=/; Domain=localhost; SameSite=Lax; Max-Age=3600; Expires=Mon, 02 Mar 2026 17:00:00 GMT; HttpOnly; Secure'
  for (const [before, after] of [['a=b', 'wrong'], ['Path=/', 'Path=/wrong'], ['Domain=localhost', 'Domain=wrong'], ['Lax', 'Strict'], ['3600', '1'], ['17:00:00', '16:00:01'], ['; HttpOnly', ''], ['; Secure', '']]) {
    assert.notEqual(describe('set-cookie', original), describe('set-cookie', original.replace(before, after)))
  }
  assert.match(describe('set-cookie', original), /preference=a=b/)
})
test('session cookie bytes normalize; presence, clearing and attributes do not', () => {
  for (const name of ['_campfire_session', 'session_token']) {
    assert.equal(describe('set-cookie', `${name}=a; Path=/`), describe('set-cookie', `${name}=b; Path=/`))
    assert.notEqual(describe('set-cookie', `${name}=a; Path=/`), describe('set-cookie', `${name}=; Path=/`))
    assert.notEqual(describe('set-cookie', `${name}=a; Path=/`), describe('set-cookie', `${name}=b; Path=/wrong`))
  }
  assert.notEqual(describe('set-cookie', 'two_factor_remembered_device=a'), describe('set-cookie', 'two_factor_remembered_device=b'))
})
test('Rack HTML ETags normalize only after proving the validator names the response bytes', () => {
  const make = (token: string, etag?: string) => {
    const body = Buffer.from(`<meta name="csrf-token" content="${token}">`)
    return responseHeaders([{ name: 'content-type', value: 'text/html' }, { name: 'etag', value: etag ?? `W/"${createHash('sha256').update(body).digest('hex').slice(0, 32)}"` }], options, body)
  }
  assert.equal(make('a'), make('b'))
  assert.notEqual(make('a'), make('b', 'W/"00000000000000000000000000000000"'))
  assert.notEqual(make('a'), make('b', '"00000000000000000000000000000000"'))
})
test('cache instrumentation normalizes only known hit/miss values', () => {
  assert.equal(describe('x-cache', 'hit'), describe('x-cache', 'miss'))
  assert.notEqual(describe('x-cache', 'hit'), describe('x-cache', 'wrong'))
  assert.notEqual(describe('x-cache', 'hit'), responseHeaders([], options))
})
test('WebKit comma splitting reconstructs dates and policy fields, never cookie pairs', () => {
  const complete = responseHeaders([{ name: 'date', value: 'Mon, 02 Mar 2026 16:00:00 GMT' }, { name: 'permissions-policy', value: 'camera=(self), microphone=(self)' }, { name: 'set-cookie', value: 'a=1; Expires=Mon, 02 Mar 2026 17:00:00 GMT' }, { name: 'set-cookie', value: 'b=2' }], options)
  const split = responseHeaders([{ name: 'date', value: 'Mon' }, { name: 'date', value: '02 Mar 2026 16:00:00 GMT' }, { name: 'permissions-policy', value: 'camera=(self)' }, { name: 'permissions-policy', value: 'microphone=(self)' }, { name: 'set-cookie', value: 'a=1; Expires=Mon, 02 Mar 2026 17:00:00 GMT' }, { name: 'set-cookie', value: 'b=2' }], options)
  assert.equal(complete, split)
  assert.match(complete, /set-cookie: a=1/); assert.match(complete, /set-cookie: b=2/)
})
