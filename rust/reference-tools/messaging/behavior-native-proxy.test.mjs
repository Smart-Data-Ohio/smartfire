import assert from 'node:assert/strict';
import {test} from 'node:test';
import http from 'node:http';
import {nativeAssetProxy} from './behavior-native-proxy.mjs';
test('native served fault changes only its real CSS response and always closes',async()=>{
  const app=http.createServer((request,response)=>{response.setHeader('content-type','text/css');response.end(request.method==='POST'?'written':'.message__quick-reaction { color: red; }');});
  await new Promise(resolve=>app.listen(0,'127.0.0.1',resolve));
  const base=`http://127.0.0.1:${app.address().port}`,probe={applied:0,networkFailures:[]};
  const proxy=await nativeAssetProxy(base,['messages-','.message__quick-reaction {','FAULT .message__quick-reaction {'],probe);
  const send=(path,method='GET')=>new Promise((resolve,reject)=>{const request=http.request(proxy.url,{method,path:base+path},response=>{let body='';response.on('data',chunk=>body+=chunk);response.on('end',()=>resolve(body));});request.on('error',reject);request.end();});
  try {
    assert.match(await send('/assets/messages-abc.css'),/^FAULT/);
    assert.equal(await send('/messages','POST'),'written');
    assert.equal(await send('/assets/other.css'),'.message__quick-reaction { color: red; }');
    assert.equal(probe.applied,1);assert.deepEqual(probe.networkFailures,[]);
  }finally{await proxy.close();await new Promise(resolve=>app.close(resolve));}
  assert.equal(app.listening,false);
});
test('CONNECT preserves the Cable/browser tunnel and releases its sockets',async()=>{
  const app=http.createServer((request,response)=>response.end('tunnel preserved'));
  await new Promise(resolve=>app.listen(0,'127.0.0.1',resolve));
  const base=`http://127.0.0.1:${app.address().port}`,proxy=await nativeAssetProxy(base,['messages-','needle','fault'],{applied:0,networkFailures:[]});
  try {
    const body=await new Promise((resolve,reject)=>{
      const request=http.request(proxy.url,{method:'CONNECT',path:new URL(base).host});
      request.on('connect',(response,socket)=>{let body='';socket.on('data',chunk=>body+=chunk);socket.on('end',()=>resolve(body));socket.on('error',reject);socket.write('GET /cable HTTP/1.1\r\nHost: '+new URL(base).host+'\r\nConnection: close\r\n\r\n');});
      request.on('error',reject);request.end();
    });assert.match(body,/tunnel preserved/);
  }finally{await proxy.close();await new Promise(resolve=>app.close(resolve));}
});
test('native JavaScript fault leaves the real metadata and writes intact',async()=>{
  const app=http.createServer((request,response)=>{
    response.setHeader('content-type',request.url.endsWith('.js')?'text/javascript':'application/json');
    response.end(request.url.endsWith('.js')?'this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS':'{"saved":true}');
  });
  await new Promise(resolve=>app.listen(0,'127.0.0.1',resolve));
  const base=`http://127.0.0.1:${app.address().port}`,probe={applied:0,networkFailures:[]};
  const proxy=await nativeAssetProxy(base,['controllers/message_list_controller-','this.#suppressClickUntil = Date.now() + SUPPRESS_CLICK_MS','this.#suppressClickUntil = 0'],probe);
  const send=(path,method='GET')=>new Promise((resolve,reject)=>{const request=http.request(proxy.url,{method,path:base+path},response=>{let body='';response.on('data',chunk=>body+=chunk);response.on('end',()=>resolve(body));});request.on('error',reject);request.end();});
  try {
    assert.equal(await send('/assets/controllers/message_list_controller-abc.js'),'this.#suppressClickUntil = 0');
    assert.equal(await send('/rooms/1/messages/2/actions'),'{"saved":true}');
    assert.equal(await send('/rooms/1/messages','POST'),'{"saved":true}');
    assert.equal(probe.applied,1);assert.deepEqual(probe.networkFailures,[]);
  }finally{await proxy.close();await new Promise(resolve=>app.close(resolve));}
});
