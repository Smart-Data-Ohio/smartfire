// The original Node event harness, fed bytes from the actual PWA HTTP action.
import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { fileURLToPath } from 'node:url'
import { network } from './original_browser_network.mjs'
const proxy=await network(process.env.WS11UI_BROWSER_URL)
const response=await fetch(process.env.WS11UI_BROWSER_URL+'/service-worker.js')
assert.equal(response.status,200,'worker HTTP transport')
let source=await response.text()
if(process.env.WS11UI_BROWSER_CONTROL==='1') {
 const before='return url.pathname === OFFLINE_URL || url.pathname.startsWith("/assets/")'
 assert.equal(source.split(before).length,2,'control source guard')
 source=source.replace(before,'return true')
 console.log('ORIGINAL_MUTATION actual worker caches authenticated responses')
}
const result=spawnSync(process.execPath,[fileURLToPath(new URL('service_worker_harness.mjs',import.meta.url))],{input:source,encoding:'utf8',timeout:15000})
assert.ok(result.status!==null,'INVALID_CONTROL Node did not complete')
assert.equal(result.status,0,result.stderr+result.stdout) // Rails pwa_controller_test.rb:58
console.log('ORIGINAL_ASSERTION test/controllers/pwa_controller_test.rb:58')
assert.ok(result.stdout.includes('service worker harness: all checks passed')) // Rails :59
console.log('ORIGINAL_ASSERTION test/controllers/pwa_controller_test.rb:59')
process.stdout.write(result.stdout)
console.log('ORIGINAL_CASE served-worker: passed')
console.log('ORIGINAL_STATE_REQUESTS []')

proxy.close();
