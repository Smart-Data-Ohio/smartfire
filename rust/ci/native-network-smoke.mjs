// Check the real native-driver prerequisite before running the paired suite.
import assert from 'node:assert/strict';
import {mkdtempSync,openSync,closeSync,rmSync,readFileSync,mkdirSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {nativeNetwork} from '../reference-tools/messaging/behavior-native-network.mjs';

console.log(execFileSync('unshare',['--version'],{encoding:'utf8'}).trim());
const scratch=process.env.WS8BM_BROWSER_SCRATCH||process.env.TMPDIR;
mkdirSync(scratch,{recursive:true});
const temp=mkdtempSync(scratch+'/native-');
const logPath=temp+'/driver.log',log=openSync(logPath,'w');
let network;
try {
  network=await nativeNetwork(temp,[],log);
  const deadline=Date.now()+15000;
  for(;;) {
    let response;
    try {response=await fetch(`http://127.0.0.1:${network.port}/status`);}catch{}
    if(response?.ok) {
      assert.equal((await response.json()).value.ready,true);
      break;
    }
    if(Date.now()>=deadline) throw new Error('Native ChromeDriver readiness timeout');
    await new Promise(resolve=>setTimeout(resolve,50));
  }
  console.log('Native network smoke: 1 passed; real isolated ChromeDriver HTTP endpoint ready');
} catch(error) {
  console.error('Native driver startup log:\n'+readFileSync(logPath,'utf8'));
  throw error;
} finally {
  await network?.close();closeSync(log);rmSync(temp,{recursive:true,force:true});
}
