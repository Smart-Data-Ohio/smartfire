// Native Capybara executes the committed, byte-identical pinned declaration.
// Reuse the original-browser tooling's isolated HTTP/Cable/Selenium transport.
import assert from 'node:assert/strict';
import {spawn,spawnSync} from 'node:child_process';
import {mkdirSync,mkdtempSync,rmSync,openSync,closeSync,readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {basename,dirname} from 'node:path';
import {nativeNetwork} from '../messaging/behavior-native-network.mjs';
import {nativeAssetProxy} from '../messaging/behavior-native-proxy.mjs';
const root=fileURLToPath(new URL('../../',import.meta.url));
const tools=fileURLToPath(new URL('./',import.meta.url));
const [base,database,key,control]=process.argv.slice(2);
assert.ok(base&&database&&key&&control);
const manifest=JSON.parse(readFileSync(tools+'original_browser/manifest.json','utf8'));
const record=manifest.records.find(row=>row.id===key);
assert.ok(record,`unknown original browser declaration ${key}`);
const cache=process.env.WS14_BROWSER_SCRATCH||'/home/riels/.cache/rust-port/ws14-browser';
mkdirSync(cache,{recursive:true});
const temp=mkdtempSync(cache+'/s-'),container='ws14-original-'+basename(temp);
let network,proxy,log;
const probe={networkFailures:[]};
try {
  log=openSync(control+'/chromedriver.log','a');
  const mutation=process.env.WS14_BROWSER_MUTATION?JSON.parse(process.env.WS14_BROWSER_MUTATION):null;
  if(mutation)proxy=await nativeAssetProxy(base,mutation,probe,{sameOriginOnly:true});
  network=await nativeNetwork(temp,[Number(new URL(base).port),...(proxy?[Number(new URL(proxy.url).port)]:[])],log);
  const deadline=Date.now()+15000;
  for(;;){try{if((await fetch(`http://127.0.0.1:${network.port}/status`)).ok)break;}catch{}
    assert.ok(Date.now()<deadline,'original-browser ChromeDriver startup');
    await new Promise(resolve=>setTimeout(resolve,50));}
  const args=['run','--name',container,'--rm','--network','host','--cpus','1','--user',`${process.getuid()}:${process.getgid()}`,
    '--env-file',root+'parity/.env.reference','-e','RAILS_ENV=test','-e','GOOGLE_CLIENT_ID=test-client-id','-e','GOOGLE_CLIENT_SECRET=test-client-secret',
    '-e',`WS14_BROWSER_BASE=${base}`,'-e',`WS14_BROWSER_DRIVER_PORT=${network.port}`,'-e',`WS14_BROWSER_CASE=${key}`,
    '-e',`WS14_BROWSER_DATABASE=/readback/${basename(database)}`,'-e','WS14_BROWSER_CONTROL=/control',
    ...(proxy?['-e',`WS14_BROWSER_PROXY=${proxy.url}`]:[]),
    '-v',`${dirname(database)}:/readback:rw`,'-v',`${control}:/control:rw`,'-v',`${tools}:/tools:ro`,
    '-v',`${tools}original_browser/test/support/test_session_controller.rb:/rails/test/support/test_session_controller.rb:ro`,
    '--entrypoint','bundle',process.env.WS14_BROWSER_RUBY_IMAGE||process.env.PARITY_IMAGE||'campfire-reference','exec','ruby','/tools/ws14_original_browser.rb','--name',record.test];
  const result=await new Promise((resolve,reject)=>{
    const child=spawn('docker',args);let stdout='',stderr='';
    const timer=setTimeout(()=>{child.kill('SIGTERM');reject(new Error('original browser declaration timeout'));},180000);
    child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);
    child.on('error',error=>{clearTimeout(timer);reject(error);});
    child.on('exit',(status,signal)=>{clearTimeout(timer);resolve({status,signal,stdout,stderr});});
  });
  process.stdout.write(result.stdout);process.stderr.write(result.stderr);
  console.log('WS14_ORIGINAL_TRANSPORT '+JSON.stringify({id:key,mutationApplied:probe.applied||0,networkFailures:probe.networkFailures}));
  assert.equal(result.status,0,`${key}: pinned original browser assertion failed`);
  assert.match(result.stdout,/1 runs, [1-9]\d* assertions, 0 failures, 0 errors, 0 skips/,'exactly one original declaration ran assertions');
  assert.match(result.stdout,new RegExp(`WS14_ORIGINAL_RECEIPT ${key} `),'per-declaration receipt');
  const receipt=JSON.parse(result.stdout.match(new RegExp(`WS14_ORIGINAL_RECEIPT ${key} (.+)`))[1]);
  for(const line of record.assertion_lines)assert.ok(receipt.lines.includes(line),`${key}: original assertion at ${record.file}:${line} executed`);
  for(const helper of record.helper_assertions)assert.ok(receipt.files[helper.file]?.includes(helper.line),`${key}: original helper assertion at ${helper.file}:${helper.line} executed`);
}finally{
  spawnSync('docker',['rm','-f',container],{stdio:'ignore'});
  if(proxy)await proxy.close();
  if(network)await network.close();
  if(log!==undefined)closeSync(log);
  rmSync(temp,{recursive:true,force:true});
}
