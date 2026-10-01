import assert from 'node:assert/strict';
import {test, before, after} from 'node:test';
import {chromium} from 'playwright';
import {pollBrowser} from './ws13-browser-poll.mjs';
let browser;
before(async()=>{browser=await chromium.launch({headless:true});});
after(async()=>{await browser?.close();});
test('an asynchronous false RTP sample cannot signal media readiness',async t=>{
  const page=await browser.newPage();t.after(()=>page.close());
  await page.evaluate(()=>{window.samples=0;});
  await assert.rejects(pollBrowser(page,async()=>{
    window.samples++;
    return await Promise.resolve(false);
  },null,{timeout:250,polling:10,message:'no RTP'}),/no RTP/);
  assert.ok(await page.evaluate(()=>window.samples)>1);
});
test('media readiness waits for a completed positive RTP sample',async t=>{
  const page=await browser.newPage();t.after(()=>page.close());
  await page.evaluate(()=>{window.samples=0;});
  const result=await pollBrowser(page,async()=>{
    const sample=++window.samples;
    await new Promise(resolve=>setTimeout(resolve,10));
    return sample>=3;
  },null,{timeout:1000,polling:10,message:'no RTP'});
  assert.equal(result,true);
  assert.equal(await page.evaluate(()=>window.samples),3);
});
