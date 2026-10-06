// Stable namespace: other workers' Docker bridge changes cannot abort Chromium.
import fs from 'node:fs'
import net from 'node:net'
import { connectUpstream } from '../../parity/capture/forward.ts'
export async function network(base) {
 if(!process.env.PARITY_UPSTREAM_SOCKET)throw Error('browser forwarder is required')
 if(fs.readlinkSync('/proc/self/ns/net')===process.env.WS11UI_HOST_NETWORK)throw Error('browser must have an isolated network namespace')
 const origin=new URL(base)
 const proxy=net.createServer(client=>{
  client.pause()
  const upstream=connectUpstream(origin.hostname,Number(origin.port),()=>{client.pipe(upstream);upstream.pipe(client);client.resume()})
  const close=()=>{client.destroy();upstream.destroy()}
  client.on('error',close);upstream.on('error',close)
  client.on('close',close);upstream.on('close',close)
 })
 await new Promise(resolve=>proxy.listen(Number(origin.port),origin.hostname,resolve))
 console.log('Original browser network: isolated namespace, unchanged HTTP/WebSocket bytes')
 return proxy
}
