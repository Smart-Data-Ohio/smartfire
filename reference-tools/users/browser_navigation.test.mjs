import assert from 'node:assert/strict'
import http from 'node:http'
import {test} from 'node:test'
import {chromium} from 'playwright'
import {visit, waitForController} from './browser_navigation.mjs'
import {fillThreadName} from './thread_form.mjs'

test('initial navigation reaches the UI while an unrelated image is pending', async t => {
  let imageStarted
  const pending = new Promise(resolve => { imageStarted = resolve })
  const server = http.createServer((request, response) => {
    if (request.url === '/pending-image') {
      imageStarted()
      return // Deliberately hold the image; the document and UI are ready.
    }
    response.writeHead(200, {'content-type':'text/html'})
    response.end('<button id="profile">Profile</button><img src="/pending-image">')
  })
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve))
  const browser = await chromium.launch({headless:true, args:['--no-sandbox']})
  t.after(async () => {
    await browser.close()
    server.closeAllConnections()
    await new Promise(resolve => server.close(resolve))
  })
  const page = await browser.newPage()
  page.setDefaultTimeout(2000)
  const response = await visit(page, `http://127.0.0.1:${server.address().port}`)
  await pending
  assert.equal(response.status(), 200)
  assert.equal(await page.locator('#profile').isVisible(), true)
  assert.equal(await page.evaluate(() => document.readyState), 'interactive')
})

test('visible checkboxes do not signal that their controller has connected', async t => {
  const browser = await chromium.launch({headless:true, args:['--no-sandbox']})
  t.after(() => browser.close())
  const page = await browser.newPage()
  page.setDefaultTimeout(2000)
  await page.setContent(`<div data-controller="multi-select">
    <input type="checkbox" id="member"><button id="message">Message (0)</button>
    </div><script>
      window.controllerLookups = 0; window.controllerConnected = false;
      window.Stimulus = {getControllerForElementAndIdentifier() {
        window.controllerLookups++;
        return window.controllerConnected ? {} : null;
      }};
    </script>`)
  assert.equal(await page.locator('#member').isVisible(), true)
  let resolved = false
  const connection = waitForController(page, 'multi-select').then(() => { resolved = true })
  // Observe an actual readiness probe before connecting, rather than a sleep.
  await page.waitForFunction(() => window.controllerLookups > 0)
  assert.equal(resolved, false)
  await page.evaluate(() => {
    document.querySelector('#member').addEventListener('change', () => {
      document.querySelector('#message').textContent = 'Message (1)'
    })
    window.controllerConnected = true
  })
  await connection
  await page.locator('#member').check()
  assert.equal(await page.locator('#message').textContent(), 'Message (1)')
})

test('thread name filling waits for the original form focus prerequisite', async t => {
  const browser = await chromium.launch({headless:true, args:['--no-sandbox']})
  t.after(() => browser.close())
  const page = await browser.newPage()
  page.setDefaultTimeout(2000)
  await page.setContent(`<div id="thread-panel">
    <input data-thread-panel-target="createName">
    <textarea data-thread-panel-target="createMessage"></textarea>
    </div>`)
  await page.evaluate(() => {
    const active = Object.getOwnPropertyDescriptor(Document.prototype, 'activeElement').get
    window.focusProbes = 0
    Object.defineProperty(document, 'activeElement', {get() {
      window.focusProbes++
      return active.call(document)
    }})
  })
  const writing = fillThreadName(page, 'Agent owned thread')
  await page.waitForFunction(() => window.focusProbes > 0)
  assert.equal(await page.locator('input').inputValue(), '')
  // Release the focus callback explicitly; no timer or retry makes it ready.
  await page.locator('textarea').focus()
  await writing
  assert.equal(await page.locator('input').inputValue(), 'Agent owned thread')
})
