import assert from 'node:assert/strict';
import {test} from 'node:test';
import {EventEmitter} from 'node:events';
import {running,stopChild} from './behavior-native-child.mjs';
test('a driver already ended by a signal is not awaited or killed again',async()=>{
  const child=new EventEmitter();child.exitCode=null;child.signalCode='SIGTERM';
  child.kill=()=>assert.fail('already exited');
  assert.equal(running(child),false);
  await stopChild(child);
  assert.equal(child.listenerCount('exit'),0);
});
test('live driver cleanup subscribes before termination',async()=>{
  const child=new EventEmitter();child.exitCode=null;child.signalCode=null;
  child.kill=signal=>{assert.equal(signal,'SIGTERM');assert.equal(child.listenerCount('exit'),1);child.signalCode=signal;child.emit('exit',null,signal);};
  await stopChild(child);assert.equal(child.listenerCount('exit'),0);
});
