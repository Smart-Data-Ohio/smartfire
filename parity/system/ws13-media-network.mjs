// Keep Chromium in its own stable network namespace. Host Docker veth changes
// otherwise cancel lazy controller fetches with ERR_NETWORK_CHANGED. Forward
// opaque native ICE/DTLS/SRTP datagrams to the same loopback LiveKit server.
import net from 'node:net';
import dgram from 'node:dgram';
import fs from 'node:fs';
function connectUpstream(port,onConnect) {
  const stream=net.connect(process.env.PARITY_UPSTREAM_SOCKET,()=>{
    stream.write(`127.0.0.1:${port}\n`);onConnect();
  });
  return stream;
}

function sendFrame(stream,port,packet) {
  const frame=Buffer.allocUnsafe(6+packet.length);
  frame.writeUInt32BE(packet.length,0);frame.writeUInt16BE(port,4);packet.copy(frame,6);
  stream.write(frame);
}
function frames(stream,receive) {
  let pending=Buffer.alloc(0);
  stream.on('data',chunk=>{
    pending=Buffer.concat([pending,chunk]);
    while(pending.length>=6) {
      const length=pending.readUInt32BE(0);
      if(length>65507) {stream.destroy(new Error('invalid local media datagram'));return;}
      if(pending.length<6+length)return;
      receive(pending.readUInt16BE(4),pending.subarray(6,6+length));
      pending=pending.subarray(6+length);
    }
  });
}
export async function hostMedia(socketPath,serverPort=7882) {
  const clients=new Set();fs.rmSync(socketPath,{force:true});
  const server=net.createServer(stream=>{
    clients.add(stream);const peers=new Map();
    stream.on('error',()=>{});
    stream.on('close',()=>{clients.delete(stream);for(const peer of peers.values())peer.close();});
    frames(stream,(port,packet)=>{
      let peer=peers.get(port);
      if(!peer) {
        peer=dgram.createSocket('udp4');peers.set(port,peer);
        peer.on('error',error=>stream.destroy(error));
        peer.on('message',(packet,remote)=>{if(remote.address==='127.0.0.1'&&remote.port===serverPort)sendFrame(stream,port,packet);});
        peer.bind(0,'127.0.0.1');
      }
      peer.send(packet,serverPort,'127.0.0.1');
    });
  });
  await new Promise(resolve=>server.listen(socketPath,resolve));fs.chmodSync(socketPath,0o666);
  return async()=>{
    for(const client of clients)client.destroy();
    await new Promise(resolve=>server.close(resolve));fs.rmSync(socketPath,{force:true});
  };
}
export async function browserMedia(socketPath,{udpPort=7882,tcpPorts=[]}={}) {
  const stream=net.connect(socketPath);await new Promise((resolve,reject)=>{stream.once('connect',resolve);stream.once('error',reject);});
  const udp=dgram.createSocket('udp4');
  const addresses=new Map();let sent=0,received=0,closed=false;
  stream.on('error',error=>udp.emit('error',error));
  udp.on('error',error=>{console.error('local media relay:',error.message);stream.destroy();});
  udp.on('message',(packet,remote)=>{addresses.set(remote.port,remote.address);sent++;sendFrame(stream,remote.port,packet);});
  frames(stream,(port,packet)=>{if(!closed){received++;udp.send(packet,port,addresses.get(port));}});
  await new Promise((resolve,reject)=>{udp.once('error',reject);udp.bind(udpPort,'127.0.0.1',resolve);});
  const servers=[];const connections=new Set();
  for(const port of tcpPorts) {
    const server=net.createServer(client=>{
      const upstream=connectUpstream(port,()=>{client.pipe(upstream);upstream.pipe(client);});
      connections.add(client);connections.add(upstream);
      const close=()=>{client.destroy();upstream.destroy();connections.delete(client);connections.delete(upstream);};
      for(const socket of [client,upstream]) {socket.on('error',close);socket.on('close',close);}
    });
    await new Promise(resolve=>server.listen(port,'127.0.0.1',resolve));servers.push(server);
  }
  const close=async()=>{
    closed=true;
    for(const connection of connections)connection.destroy();
    for(const server of servers)await new Promise(resolve=>server.close(resolve));
    await new Promise(resolve=>udp.close(resolve));stream.destroy();
    console.log(`WS13 local media datagrams: ${sent} sent; ${received} received`);
  };
  close.port=udp.address().port;
  return close;
}
if(import.meta.url===`file://${process.argv[1]}`) {
  const [mode,socketPath,...ports]=process.argv.slice(2);
  const close=mode==='host'?await hostMedia(socketPath):await browserMedia(socketPath,{tcpPorts:ports.map(Number)});
  const stop=async()=>{await close();process.exit(0);};
  process.on('SIGTERM',stop);process.on('SIGINT',stop);
}
