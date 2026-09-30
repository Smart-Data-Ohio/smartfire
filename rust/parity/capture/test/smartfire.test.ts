import { test } from 'node:test'
import assert from 'node:assert/strict'
import { normalizeDocument } from '../normalize.ts'
import { expandJobs } from '../inventory.ts'
import { retryableCaptureError, run } from '../run.ts'
import { checkStatus } from '../capture.ts'
import { NetworkLog } from '../network.ts'
import fs from 'node:fs'
import { PARITY_DIR } from '../config.ts'
import path from 'node:path'

fs.mkdirSync(path.join(PARITY_DIR, 'out'), { recursive: true })

test('CSRF fields are compared; only their random values differ', () => {
  const html = (token: string) => `<meta name="csrf-param" content="authenticity_token"><meta name="csrf-token" content="${token}"><form action="/session"><input name="authenticity_token" value="${token}" type="hidden"></form>`
  assert.notEqual(normalizeDocument(html('a')), normalizeDocument('<form action="/session"></form>'))
  assert.equal(normalizeDocument(html('a')), normalizeDocument(html('b')))
  assert.match(normalizeDocument(html('a')), /name="authenticity_token"/)
})

test('lean captures each route once, with explicit mobile and dark smoke states', () => {
  const jobs = expandJobs([{ id: 'boards/index', path: '/rooms/boards', seed: 'default', steps: [] }], { matrix: 'lean', breakpoints: 'exclude' }, {} as any)
  assert.deepEqual(jobs.map(j => [j.cell.engine, j.cell.viewport.name, j.cell.scheme]), [['chromium', 'desktop', 'light']])
})

test('deterministic HTTP and selector failures are never retried', () => {
  assert.equal(retryableCaptureError('HTTP 404 at /work'), false)
  assert.equal(retryableCaptureError('not ready after 5000ms: controllers not registered: composer'), false)
  assert.equal(retryableCaptureError('locator.click: Timeout 5000ms exceeded'), false)
  assert.equal(retryableCaptureError('Target page, context or browser has been closed'), true)
})

test('an empty capture selection cannot pass the gate', async () => {
  const outDir = fs.mkdtempSync(path.join(PARITY_DIR, 'out/ws19-empty-'))
  try {
    await assert.rejects(run({
      states: [], filter: { matrix: 'lean' }, targets: [], outDir,
      time: '2026-03-02T16:00:00Z', timeoutMs: 8000, quiet: true,
    } as any), /no selected inventory cells/)
  } finally { fs.rmSync(outDir, { recursive: true, force: true }) }
})

test('a form rejection checks its initial and submitted document statuses separately', () => {
  const state = { id: 'auth/sudo_rejected', path: '/sudo/new', seed: 'default', steps: [], expect_final_status: 401 }
  assert.doesNotThrow(() => checkStatus(state, { status: 200 } as any))
  assert.doesNotThrow(() => checkStatus(state, { status: 401 } as any, true))
  assert.throws(() => checkStatus(state, { status: 200 } as any, true), /expected HTTP 401/)
  assert.throws(() => checkStatus(state, { status: 404 } as any), /expected HTTP 200/)
})

import { EventEmitter } from 'node:events'
import { PageTracker } from '../readiness.ts'
test('a missing JavaScript resource fails readiness while an image denial remains observable', () => {
  const page = new EventEmitter()
  const tracker = new PageTracker(page as any)
  const response = (type: string) => ({ status: () => 404, url: () => `http://localhost:3999/${type}`, request: () => ({ method: () => 'GET', resourceType: () => type }) })
  page.emit('response', response('image'))
  assert.deepEqual(tracker.fatalResources, [])
  page.emit('response', response('script'))
  assert.deepEqual(tracker.fatalResources, ['HTTP 404 http://localhost:3999/script'])
})

test('interaction assertions distinguish a rejected POST from a successful GET', async () => {
  const page = new EventEmitter()
  const log = new NetworkLog(page as any, 'http://localhost:3999', {})
  const response = (method: string, status: number) => ({
    url: () => 'http://localhost:3999/session', status: () => status,
    request: () => ({ url: () => 'http://localhost:3999/session', method: () => method, resourceType: () => 'fetch', isNavigationRequest: () => false }),
    allHeaders: async () => ({ 'content-type': 'application/json' }), headersArray: async () => [],
    body: async () => Buffer.from('{}'),
  })
  page.emit('response', response('GET', 200))
  assert.equal(log.hasResponse('POST', '/session', 401), false)
  page.emit('response', response('POST', 401))
  assert.equal(log.hasResponse('POST', '/session', 401), true)
  assert.equal(log.hasResponse('POST', '/session', 200), false)
  await log.text()
})

test('Action Cable confirms one identifier even when two client subscriptions use it', () => {
  const page = new EventEmitter()
  const socket = new EventEmitter()
  const tracker = new PageTracker(page as any)
  page.emit('websocket', socket)
  const identifier = JSON.stringify({ channel: 'ActivityChannel' })
  const send = (command: string) => socket.emit('framesent', { payload: JSON.stringify({ command, identifier }) })
  send('subscribe'); send('subscribe')
  socket.emit('framereceived', { payload: JSON.stringify({ type: 'confirm_subscription', identifier }) })
  assert.equal(tracker.outstanding.get(identifier), 0)
  send('subscribe')
  assert.equal(tracker.outstanding.get(identifier), 0)
  send('unsubscribe'); send('subscribe')
  assert.equal(tracker.outstanding.get(identifier), 1)
})
