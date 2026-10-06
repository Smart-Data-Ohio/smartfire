// Tools-only interleaving of the original direct-model fixtures with the browser.
import fs from 'node:fs'
import path from 'node:path'
import assert from 'node:assert/strict'
let sequence=0
export async function fixtureAction(data) {
 const directory=path.dirname(process.env.WS11UI_BROWSER_DATABASE)
 sequence=Math.max(sequence,fs.existsSync(path.join(directory,'ledger-request.json'))?JSON.parse(fs.readFileSync(path.join(directory,'ledger-request.json'))).sequence:0)+1
 const tmp=path.join(directory,'ledger-request.tmp')
 fs.writeFileSync(tmp,JSON.stringify({...data,sequence}));fs.renameSync(tmp,path.join(directory,'ledger-request.json'))
 const start=performance.now()
 while(performance.now()-start<10000) {
  const response=path.join(directory,'ledger-response.json')
  if(fs.existsSync(response)) {
   const answer=JSON.parse(fs.readFileSync(response,'utf8'))
   if(answer.sequence===sequence) {assert.equal(answer.error,undefined,'original fixture bridge');return answer.result}
  }
  await new Promise(resolve=>setTimeout(resolve,20))
 }
 throw Error('original fixture bridge did not answer before 10s')
}
