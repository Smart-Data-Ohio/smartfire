import assert from 'node:assert/strict';
import {test} from 'node:test';
import dgram from 'node:dgram';
import fs from 'node:fs/promises';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import {hostMedia,browserMedia} from './ws13-media-network.mjs';

test('local media relay preserves opaque datagrams and independent peer replies',async t=>{
  const scratch=process.env.TMPDIR||fileURLToPath(new URL('../../../.scratch/',import.meta.url));
  await fs.mkdir(scratch,{recursive:true});
  const dir=await fs.mkdtemp(path.join(scratch,'ws13-media-relay-'));
  // Linux Unix-domain socket addresses have a 108-byte limit. A fresh clone's
  // absolute scratch path can exceed it; both ends resolve this relative name.
  const previous=process.cwd();process.chdir(dir);
  const server=dgram.createSocket('udp4');
  const sourcePorts=new Set();const received=[];
  server.on('message',(packet,remote)=>{sourcePorts.add(remote.port);received.push(Buffer.from(packet));server.send(packet,remote.port,remote.address);});
  await new Promise(resolve=>server.bind(0,'127.0.0.1',resolve));
  const socket='media.sock';
  const stopHost=await hostMedia(socket,server.address().port);
  const stopBrowser=await browserMedia(socket,{udpPort:0});
  t.after(async()=>{await stopBrowser();await stopHost();await new Promise(resolve=>server.close(resolve));process.chdir(previous);await fs.rm(dir,{recursive:true});});
  const packets=[Buffer.from(Array.from({length:1200},(_,i)=>i%256)),Buffer.alloc(1200,255)];
  await Promise.all(packets.map(async packet=>{
    const client=dgram.createSocket('udp4');
    const echoed=new Promise((resolve,reject)=>{client.once('message',resolve);client.once('error',reject);});
    await new Promise(resolve=>client.bind(0,'127.0.0.1',resolve));
    try{client.send(packet,stopBrowser.port,'127.0.0.1');assert.deepEqual(await echoed,packet);}finally{client.close();}
  }));
  assert.equal(sourcePorts.size,2,'each native peer keeps its own server-side source port');
  assert.deepEqual(received.map(packet=>packet.toString('hex')).sort(),packets.map(packet=>packet.toString('hex')).sort(),'server receives each original opaque datagram unchanged');
});
