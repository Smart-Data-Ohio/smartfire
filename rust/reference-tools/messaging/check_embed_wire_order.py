#!/usr/bin/env python3
"""Observe actual Rails Redis/Cable delivery for the queued-child oracle (stdlib only).

Start the pinned server and run embed_wire_order.rb setup first. Publication order
must match the existing oracle; wire frames must match exactly including stream
identifier, body and multiplicity. No HTML masks, retries or server modifications.
"""
import argparse, base64, collections, hashlib, json, os, random, socket, struct, subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
p=argparse.ArgumentParser(description=__doc__)
p.add_argument('--port', type=int, required=True)
p.add_argument('--seed', type=int, required=True)
p.add_argument('--output', type=Path, required=True)
a=p.parse_args()
setup=json.loads((ROOT/f'parity/.seed/.instances/{a.port}/storage/embed-wire-setup.json').read_text())
s=socket.create_connection(('127.0.0.1',a.port),timeout=30)
key=base64.b64encode(os.urandom(16)).decode()
s.sendall((f'GET /cable HTTP/1.1\r\nHost: campfire.test\r\nOrigin: http://campfire.test\r\nCookie: {setup["cookie"]}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\nSec-WebSocket-Protocol: actioncable-v1-json\r\n\r\n').encode())
headers=b''
while not headers.endswith(b'\r\n\r\n'): headers+=s.recv(1)
assert headers.startswith(b'HTTP/1.1 101'),headers
accept=base64.b64encode(hashlib.sha1((key+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode()).digest())
fields={k.strip().lower():v.strip() for line in headers.split(b'\r\n')[1:] if b':' in line for k,v in [line.split(b':',1)]}
assert fields[b'sec-websocket-accept']==accept,headers

def receive_bytes(n):
    out=b''
    while len(out)<n:
        part=s.recv(n-len(out));assert part,'closed socket';out+=part
    return out

def send(value):
    payload=json.dumps(value,separators=(',',':')).encode();mask=os.urandom(4);n=len(payload)
    size=bytes([n|128]) if n<126 else bytes([126|128])+struct.pack('!H',n)
    s.sendall(b'\x81'+size+mask+bytes(v^mask[i%4] for i,v in enumerate(payload)))

def receive():
    while True:
        first,length=receive_bytes(2);assert first==129,(first,length)
        length&=127
        if length==126:length=struct.unpack('!H',receive_bytes(2))[0]
        if length==127:length=struct.unpack('!Q',receive_bytes(8))[0]
        value=json.loads(receive_bytes(length))
        if value.get('type')!='ping':return value

assert receive()=={'type':'welcome'}
streams=list(setup['streams']);random.Random(a.seed).shuffle(streams)
identifiers={}
for stream in streams:
    identifier=json.dumps({'channel':'RoomMessagesChannel','signed_stream_name':setup['streams'][stream]},separators=(',',':'))
    identifiers[stream]=identifier;send({'command':'subscribe','identifier':identifier})
    assert receive()=={'identifier':identifier,'type':'confirm_subscription'}
corpus=json.loads((ROOT/'vectors/messaging/older_embed_children.json').read_text())
expected=[{'identifier':identifiers[f['stream']],'message':f['html']} for g in corpus['groups'] for j in g['jobs'] for f in j['frames']]
log=a.output.with_suffix('.producer.log')
with log.open('w') as handle:
    producer=subprocess.Popen(['bash',str(ROOT/'parity/bin/reference'),'runner','--port',str(a.port),str(ROOT/'reference-tools/messaging/embed_wire_order.rb'),'jobs'],env=dict(os.environ,PARITY_NAMESPACE='ws8bm2',PARITY_OWNER='ws8bm2'),stdout=handle,stderr=subprocess.STDOUT)
    actual=[]
    try:
        for _ in expected:actual.append(receive())
    finally:
        status=producer.wait(timeout=60)
        s.settimeout(0.25)
        try:
            extra=receive();raise AssertionError(f'extra frame: {extra}')
        except TimeoutError:
            pass
        finally:
            s.close()
assert status==0,log.read_text()
canon=lambda frames:collections.Counter(json.dumps(f,sort_keys=True) for f in frames)
assert canon(actual)==canon(expected),'wire frames differ in count, envelope or bytes'
a.output.write_text(json.dumps({'seed':a.seed,'expected':expected,'actual':actual},indent=2)+'\n')
print(f'WS8bm2 Rails wire seed={a.seed}: {len(actual)}/{len(expected)} exact envelopes; publication order exact; arrival order '+('identical' if actual==expected else 'DIFFERENT'))
