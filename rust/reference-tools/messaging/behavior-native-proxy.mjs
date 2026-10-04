// A real browser HTTP proxy: only the registered served asset is changed.
// WebSocket upgrades and all writes are forwarded without rewriting payloads.
import assert from 'node:assert/strict';
import http from 'node:http';
import net from 'node:net';
export async function nativeAssetProxy(base,mutation,probe) {
  const origin=new URL(base).origin,sockets=new Set(),requests=new Set();
  const server=http.createServer((incoming,outgoing)=>{
    const url=new URL(incoming.url,base);
    const receipt={method:incoming.method,path:url.pathname};(probe.transport??=[]).push(receipt);
    const request=http.request(url,{method:incoming.method,headers:{...incoming.headers,'accept-encoding':'identity'}},response=>{
      receipt.status=response.statusCode;receipt.encoding=response.headers['content-encoding']||'identity';
      const [asset,needle,replacement]=mutation;
      if(url.origin!==origin||!url.pathname.includes('/assets/'+asset)||!url.pathname.endsWith('.css')) {
        outgoing.writeHead(response.statusCode,response.headers);response.pipe(outgoing);return;
      }
      const chunks=[];response.on('data',chunk=>chunks.push(chunk));
      response.on('end',()=>{
        try {
          let body=Buffer.concat(chunks).toString();assert.ok(body.includes(needle),'native served mutation needle');
          body=body.replace(needle,replacement);probe.applied++;
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
    const upstream=net.connect(Number(endpoint.port)||80,endpoint.hostname,()=>{
      socket.write('HTTP/1.1 200 Connection Established\r\n\r\n');
      if(head.length)upstream.write(head);upstream.pipe(socket);socket.pipe(upstream);
    });
    sockets.add(upstream);upstream.on('close',()=>sockets.delete(upstream));
    upstream.on('error',error=>{if(endpoint.origin===origin)probe.networkFailures.push(String(error));socket.destroy();});
    socket.on('error',()=>upstream.destroy());socket.on('close',()=>upstream.destroy());
  });
  server.on('upgrade',(incoming,socket,head)=>{
    const url=new URL(incoming.url,base),upstream=net.connect(Number(url.port)||80,url.hostname,()=>{
      upstream.write(`${incoming.method} ${url.pathname+url.search} HTTP/${incoming.httpVersion}\r\n`+Object.entries(incoming.headers).map(([name,value])=>`${name}: ${value}\r\n`).join('')+'\r\n');
      if(head.length)upstream.write(head);upstream.pipe(socket);socket.pipe(upstream);
    });
    sockets.add(upstream);upstream.on('close',()=>sockets.delete(upstream));
    upstream.on('error',error=>{probe.networkFailures.push(String(error));socket.destroy();});
    socket.on('error',()=>upstream.destroy());socket.on('close',()=>upstream.destroy());
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve);});
  return {url:`http://127.0.0.1:${server.address().port}`,async close(){for(const request of requests)request.destroy();for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve));}};
}
