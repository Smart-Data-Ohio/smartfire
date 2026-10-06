import assert from 'node:assert/strict';
import {test, before, after} from 'node:test';
import {chromium} from 'playwright';
import {pollBrowser} from './ws13-browser-poll.mjs';
import http from 'node:http';
import {navigate} from './ws13-browser-navigation.mjs';
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
test('fixture navigation waits for the lazy controller script to load, like Selenium',async t=>{
  const server=http.createServer((req,res)=>{
    if(req.url==='/controller.js') {
      setTimeout(()=>{res.writeHead(200,{'Content-Type':'application/javascript'});res.end('window.presenceControllerConnected=true;');},150);
    } else {
      res.writeHead(200,{'Content-Type':'text/html'});
      res.end('<script>document.addEventListener("DOMContentLoaded",()=>{const s=document.createElement("script");s.src="/controller.js";document.head.append(s);});</script>');
    }
  });
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  const page=await browser.newPage();
  t.after(async()=>{await page.close();server.closeAllConnections();await new Promise(resolve=>server.close(resolve));});
  await navigate(page,'http://127.0.0.1:'+server.address().port);
  assert.equal(await page.evaluate(()=>document.readyState),'complete');
  assert.equal(await page.evaluate(()=>window.presenceControllerConnected),true);
});
