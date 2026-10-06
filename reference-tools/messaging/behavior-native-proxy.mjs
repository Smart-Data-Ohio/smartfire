// A real browser HTTP proxy: only the registered served asset is changed.
// WebSocket upgrades and all writes are forwarded without rewriting payloads.
import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
export async function nativeAssetProxy(base,mutation,probe,{sameOriginOnly=false}={}) {
  const origin=new URL(base).origin,sockets=new Set(),requests=new Set(),timers=new Set();
  const server=http.createServer((incoming,outgoing)=>{
    const url=new URL(incoming.url,base);
    if(sameOriginOnly&&url.origin!==origin) {
      probe.networkFailures.push(`unregistered proxy origin ${url.origin}`);
      outgoing.writeHead(502);outgoing.end();return;
    }
    const receipt={method:incoming.method,path:url.pathname};(probe.transport??=[]).push(receipt);
    const request=http.request(url,{method:incoming.method,headers:{...incoming.headers,'accept-encoding':'identity'}},response=>{
      receipt.status=response.statusCode;receipt.encoding=response.headers['content-encoding']||'identity';
      const [asset,needle,replacement]=mutation||[];
      const delay=Number(process.env.WS8BM_SETUP_DELAY||0);
      if(delay&&/\/messages\/\d+\/actions(?:\?|$)/.test(url.pathname)) {
        receipt.delay=delay;
        const timer=setTimeout(()=>{timers.delete(timer);outgoing.writeHead(response.statusCode,response.headers);response.pipe(outgoing);},delay);
        timers.add(timer);
        return;
      }
      if(!mutation||url.origin!==origin||!url.pathname.includes('/assets/'+asset)||!(/\.(css|js)$/.test(url.pathname))) {
        outgoing.writeHead(response.statusCode,response.headers);response.pipe(outgoing);return;
      }
      const chunks=[];response.on('data',chunk=>chunks.push(chunk));
      response.on('end',()=>{
        try {
          let body=Buffer.concat(chunks).toString();assert.ok(body.includes(needle),'native served mutation needle');
          body=body.replace(needle,replacement);probe.applied=(probe.applied||0)+1;
          const headers={...response.headers};delete headers['content-length'];delete headers['content-encoding'];
          outgoing.writeHead(response.statusCode,headers);outgoing.end(body);
        }catch(error){probe.networkFailures.push(String(error));outgoing.destroy(error);}
      });
    });
    requests.add(request);request.on('close',()=>requests.delete(request));
    request.on('error',error=>{if(url.origin===origin)probe.networkFailures.push(String(error));outgoing.destroy(error);});
    incoming.pipe(request);
  });
  server.on('connection',socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket));});
  // Chromium tunnels even ws:// connections through an HTTP proxy using
  // CONNECT. Preserve the original Cable handshake and all frames verbatim.
  server.on('connect',(incoming,socket,head)=>{
    const endpoint=new URL('http://'+incoming.url);
    if(sameOriginOnly&&endpoint.origin!==origin) {
      probe.networkFailures.push(`unregistered proxy tunnel ${endpoint.origin}`);
      socket.end('HTTP/1.1 502 Bad Gateway\r\n\r\n');return;
    }
    const upstream=net.connect(Number(endpoint.port)||80,endpoint.hostname,()=>{
      socket.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      if(head.length)upstream.write(head);upstream.pipe(socket);socket.pipe(upstream);
    });
    sockets.add(upstream);upstream.on('close',()=>sockets.delete(upstream));
    upstream.on('error',error=>{if(endpoint.origin===origin)probe.networkFailures.push(String(error));socket.destroy();});
    socket.on('error',()=>upstream.destroy());socket.on('close',()=>upstream.destroy());
  });
  server.on('upgrade',(incoming,socket,head)=>{
    const url=new URL(incoming.url,base);
    if(sameOriginOnly&&url.origin!==origin) {
      probe.networkFailures.push(`unregistered proxy upgrade ${url.origin}`);
      socket.end('HTTP/1.1 502 Bad Gateway\r\n\r\n');return;
    }
    const upstream=net.connect(Number(url.port)||80,url.hostname,()=>{
      upstream.write(`${incoming.method} ${url.pathname+url.search} HTTP/${incoming.httpVersion}\r\n`+Object.entries(incoming.headers).map(([name,value])=>`${name}: ${value}\r\n`).join('')+'\r\n');
      if(head.length)upstream.write(head);upstream.pipe(socket);socket.pipe(upstream);
    });
    sockets.add(upstream);upstream.on('close',()=>sockets.delete(upstream));
    upstream.on('error',error=>{probe.networkFailures.push(String(error));socket.destroy();});
    socket.on('error',()=>upstream.destroy());socket.on('close',()=>upstream.destroy());
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  return {url:`http://127.0.0.1:${server.address().port}`,async close(){for(const timer of timers)clearTimeout(timer);for(const request of requests)request.destroy();for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve));}};
}
