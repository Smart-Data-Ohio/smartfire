import assert from 'node:assert/strict';
import {test} from 'node:test';
import net from 'node:net';
import {relay,namespaceArguments} from './behavior-native-network.mjs';

test('native transport preserves binary bytes and closes active streams',async()=>{
  const peers=new Set();
  const upstream=net.createServer(peer=>{
    peers.add(peer);peer.on('close',()=>peers.delete(peer));peer.on('error',()=>{});
    peer.pipe(peer);
  });
  await new Promise(resolve=>upstream.listen(0,'127.0.0.1',resolve));
  let proxy,client;
  try {
    proxy=await relay({port:0,host:'127.0.0.1'},ready=>net.connect(upstream.address().port,'127.0.0.1',ready));
    client=net.connect(proxy.port,'127.0.0.1');
    const sent=Buffer.from([0,1,10,13,127,128,255]);
    const received=new Promise((resolve,reject)=>{let bytes=Buffer.alloc(0);client.on('error',reject);client.on('data',chunk=>{bytes=Buffer.concat([bytes,chunk]);if(bytes.length>=sent.length)resolve(bytes);});});
    client.write(sent);
    assert.deepEqual(await received,sent);
    const closed=new Promise(resolve=>client.once('close',resolve));
    await proxy.close();proxy=undefined;
    await closed;
  }finally {
    client?.destroy();if(proxy)await proxy.close();for(const peer of peers)peer.destroy();
    await new Promise(resolve=>upstream.close(resolve));
  }
});

test('native browser namespace cannot outlive its supervisor or namespace init',()=>{
  for(const required of ['--net','--pid','--fork','--kill-child=SIGTERM','--forward-signals'])assert.ok(namespaceArguments.includes(required));
});
