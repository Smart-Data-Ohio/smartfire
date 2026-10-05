import assert from 'node:assert/strict'
import http from 'node:http'
import {test} from 'node:test'
import {chromium} from 'playwright'
import {visit} from './browser_navigation.mjs'

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
