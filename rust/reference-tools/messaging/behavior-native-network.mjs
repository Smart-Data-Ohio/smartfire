// A stable network boundary for the behavior browsers. HTTP, Cable
// and Selenium protocol bytes cross Unix sockets unchanged; assertions do not.
import net from 'node:net';
import {spawn,execFileSync} from 'node:child_process';
import {readlinkSync,existsSync,rmSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {connectUpstream} from '../../parity/capture/forward.ts';
import {running,stopChild} from './behavior-native-child.mjs';

export async function relay(endpoint,connect) {
  const sockets=new Set();
  const server=net.createServer(client=>{
    client.pause();
    const upstream=connect(()=>{client.pipe(upstream);upstream.pipe(client);client.resume();});
    sockets.add(client);sockets.add(upstream);
    const close=()=>{client.destroy();upstream.destroy();sockets.delete(client);sockets.delete(upstream);};
    for(const socket of [client,upstream]) {socket.on('error',close);socket.on('close',close);}
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(endpoint,resolve);});
  const close=async()=>{for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve));};
  return {close,port:server.address()?.port};
}

async function waitSocket(path,child) {
  const end=Date.now()+15000;
  while(!existsSync(path)) {
    if(!running(child)) throw new Error('native network child exited before readiness');
    if(Date.now()>=end) throw new Error('native network socket startup timeout');
    await new Promise(resolve=>setTimeout(resolve,25));
  }
}

export const namespaceArguments=['--user','--map-root-user','--net','--pid','--fork','--kill-child=SIGTERM','--forward-signals'];

export async function nativeNetwork(temp,upstreamPorts,log) {
  const socket=temp+'/upstream.sock',driverSocket=temp+'/driver.sock';
  const forwardPath=fileURLToPath(new URL('../../parity/capture/forward.ts',import.meta.url));
  const wrapper=fileURLToPath(import.meta.url);
  let forward,driver,closeRelay;
  const close=async()=>{
    try {await stopChild(driver);}
    finally {
      try {if(closeRelay)await closeRelay();}
      finally {await stopChild(forward);rmSync(socket,{force:true});rmSync(driverSocket,{force:true});}
    }
  };
  try {
    forward=spawn('node',[forwardPath,socket],{stdio:['ignore',log,log]});
    await waitSocket(socket,forward);
    // PID isolation also tears down Chrome descendants when its namespace init
    // exits. unshare forwards ordinary teardown signals and kills its namespace
    // init when the unshare supervisor exits.
    driver=spawn('unshare',[...namespaceArguments,'node',wrapper,'driver',driverSocket,'9515',...upstreamPorts.map(String)],{
      stdio:['ignore',log,log],env:{...process.env,TMPDIR:temp,PARITY_UPSTREAM_SOCKET:socket,WS8BM_HOST_NETWORK:readlinkSync('/proc/self/ns/net')},
    });
    await waitSocket(driverSocket,driver);
    const endpoint=await relay({port:0,host:'127.0.0.1'},onConnect=>net.connect(driverSocket,onConnect));
    closeRelay=endpoint.close;
    console.log('WS8bm native browser network: isolated namespace; unchanged HTTP, WebSocket and Selenium bytes');
    return {driver,port:endpoint.port,close};
  }catch(error){await close();throw error;}
}

if(process.argv[2]==='driver') {
  const [, , ,socket,port,...upstreamPorts]=process.argv;
  if(process.env.WS8BM_HOST_NETWORK===readlinkSync('/proc/self/ns/net')) throw new Error('native browser must have an isolated network namespace');
  execFileSync('ip',['link','set','lo','up']);
  const closers=[];
  let child,stopping=false;
  const close=async()=>{
    if(stopping)return;stopping=true;
    try {await stopChild(child);}
    finally {for(const closer of closers.reverse())await closer();rmSync(socket,{force:true});}
    process.exit(0);
  };
  process.on('SIGTERM',close);process.on('SIGINT',close);
  for(const upstream of upstreamPorts) closers.push((await relay({port:Number(upstream),host:'127.0.0.1'},onConnect=>connectUpstream('127.0.0.1',Number(upstream),onConnect))).close);
  child=spawn('chromedriver',[`--port=${port}`,'--allowed-ips=127.0.0.1'],{stdio:'inherit'});
  child.on('error',async error=>{console.error(error);await close();});
  child.on('exit',async(status,signal)=>{if(!stopping){console.error(`native driver exited: ${status} ${signal}`);await close();}});
  closers.push((await relay(socket,onConnect=>net.connect(Number(port),'127.0.0.1',onConnect))).close);
}
