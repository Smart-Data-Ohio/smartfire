// Positive phone parity uses the actual pinned Capybara/Selenium sequence.
// No added focus assertion or diagnostic wait earns declaration credit.
import assert from 'node:assert/strict';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,mkdtempSync,rmSync,openSync,closeSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {createServer} from 'node:net';
import {createHash} from 'node:crypto';
import {basename,dirname} from 'node:path';
import {nativeAssetProxy} from './behavior-native-proxy.mjs';
import {PIN,REFERENCE_IMAGE} from './reference-pin.mjs';
export const PHONE_CASE='keeps the thread drawer usable on a phone and preserves the channel';
export function extractPhone(source) {
  const start=source.indexOf(`  test "${PHONE_CASE}" do\n`);
  const end=source.indexOf('\n  test ',start+1);
  const helpers=source.indexOf('\n  private\n');
  assert.ok(start>=0&&end>start&&helpers>end,'pinned phone source boundaries');
  const body=source.slice(start,end).replace(`test "${PHONE_CASE}" do`,'def test_phone');
  assert.equal((body.match(/save_thread_screenshot "mobile-drawer.png"/g)||[]).length,1);
  // The pixel phase is explicitly excluded. Every behavior assertion/action
  // and original wait is otherwise byte-for-byte from the pinned test body.
  return {body:body.replace('    save_thread_screenshot "mobile-drawer.png"\n',''),helpers:source.slice(helpers,source.lastIndexOf('\nend'))};
}
export async function nativePhone(base,{sourcePath='test/system/threads_test.rb',extract=extractPhone,line=293,label='phone',database,mutation,probe={}}={}) {
  const root=fileURLToPath(new URL('../../../',import.meta.url));
  const tools=fileURLToPath(new URL('./',import.meta.url));
  const scratch=root+'.scratch/ws8bm-native-phone';
  mkdirSync(scratch,{recursive:true});
  const proof=mkdtempSync(scratch+'/proof-');
  // Chrome's Unix socket path must be short. This owned cache is authorized;
  // no /tmp path or another worker's files/listeners are used.
  const cache='/home/riels/.cache/rust-port/ws8bm';mkdirSync(cache,{recursive:true});
  const temp=mkdtempSync(cache+'/s-');
  let driver,log,proxy;
  const container='ws8bm-native-'+basename(temp);
  try {
    const source=execFileSync('git',['show',`${PIN}:${sourcePath}`],{cwd:root,encoding:'utf8'});
    const {body,helpers}=extract(source);
    writeFileSync(proof+'/phone-body.rb',body);writeFileSync(proof+'/phone-helpers.rb',helpers);
    if(['video','progress'].includes(label)) {
      const name=label==='video'?'uploading a fresh video in the thread composer':'late upload progress preserves a delivered attachment and reply preview';
      line=source.split('\n').findIndex(row=>row.trim()===`test "${name}" do`)+1;
      assert.ok(line>0,'native upload source line');
    }
    writeFileSync(proof+'/native-location.json',JSON.stringify({sourcePath,line,label}));
    if(['video','progress'].includes(label)) {
      const upload=label==='video'?'alpha-centuri.mov':'moon.jpg';
      writeFileSync(temp+'/'+upload,execFileSync('git',['show',`${PIN}:test/fixtures/files/${upload}`],{cwd:root,maxBuffer:16*1024*1024}));
      writeFileSync(proof+'/application_system_test_case.rb',execFileSync('git',['show',`${PIN}:test/application_system_test_case.rb`],{cwd:root}));
    }
    writeFileSync(proof+'/system_test_helper.rb',execFileSync('git',['show',`${PIN}:test/test_helpers/system_test_helper.rb`],{cwd:root}));
    writeFileSync(proof+'/sessions.json',readFileSync(new URL('../../vectors/campfire_sessions.json',import.meta.url)));
    // Refuse an occupied port; never reuse or stop someone else's driver.
    await new Promise((resolve,reject)=>{const check=createServer();check.once('error',reject);check.listen(52023,'127.0.0.1',()=>check.close(resolve));});
    log=openSync(scratch+'/chromedriver.log','a');
    driver=spawn('chromedriver',['--port=52023','--allowed-ips=127.0.0.1'],{stdio:['ignore',log,log],env:{...process.env,TMPDIR:temp}});
    let startupError;driver.once('error',error=>{startupError=error;});
    const deadline=Date.now()+15000;
    for(;;) {
      if(startupError) throw startupError;
      if(driver.exitCode!==null) throw new Error('ChromeDriver exited during startup');
      try {if((await fetch('http://127.0.0.1:52023/status')).ok) break;}catch{}
      if(Date.now()>=deadline) throw new Error('ChromeDriver startup timeout');
      await new Promise(resolve=>setTimeout(resolve,50));
    }
    console.log(`WS8bm native ${label} source: Rails ${PIN}; SHA256 ${createHash('sha256').update(source).digest('hex')}; ${label==='attachment'?'original behavior body; shared file path adapted':label==='release'?"original behavior body/helpers; main's #composer scope retained":'unchanged behavior body/helpers, screenshots omitted'}`);
    const extra=database?['--env-file',root+'rust/parity/.env.reference','-e','RAILS_ENV=test','-e','WS8BM_NATIVE_DATABASE=/readback/'+basename(database),'-e',`WS8BM_NATIVE_UPLOAD=${temp}/markdown-workspace-attachment.txt`,'-v',`${dirname(database)}:/readback:${label==='video'?'rw':'ro'}`,'-v',`${temp}:${temp}`]:[];
    if(['video','progress'].includes(label)) {
      const storage=database.includes('/.instances/')?dirname(dirname(database))+'/storage':dirname(dirname(database))+'/files';
      extra.push('-v',`${storage}:/rails/storage/files`,'-v',`${storage}:/rails/tmp/storage`,'-e',`WS8BM_NATIVE_FIXTURES=${temp}`,'-e','WS8BM_TEST_JOB_ADAPTER=1');
    }
    if(database&&process.env.CI!==undefined) extra.push('-e',`CI=${process.env.CI}`);
    if(mutation||Number(process.env.WS8BM_SETUP_DELAY||0)) {probe.networkFailures=[];proxy=await nativeAssetProxy(base,mutation,probe);}
    const args=['run','--name',container,'--rm','--network','host','--cpus','2',...extra,'-v',`${proof}:/proof:ro`,'-v',`${tools}:/tools:ro`,'-e',`WS8BM_NATIVE_BASE=${base}`,...(proxy?['-e',`WS8BM_NATIVE_PROXY=${proxy.url}`]:[]),'--entrypoint','bundle',REFERENCE_IMAGE,'exec','ruby','/tools/behavior-native-phone.rb'];
    const result=await new Promise((resolve,reject)=>{
      const child=spawn('docker',args);let stdout='',stderr='';
      const timer=setTimeout(()=>{child.kill('SIGTERM');reject(new Error('Native browser process timeout'));},120000);
      child.stdout.on('data',chunk=>stdout+=chunk);child.stderr.on('data',chunk=>stderr+=chunk);
      child.on('error',error=>{clearTimeout(timer);reject(error);});
      child.on('exit',(status,signal)=>{clearTimeout(timer);resolve({status,signal,stdout,stderr});});
    });
    if(proxy)console.log('WS8bm native transport: '+JSON.stringify(probe.transport||[]));
    process.stdout.write(result.stdout||'');process.stderr.write(result.stderr||'');
    if(label==='attachment') captureUploadReferenceLog(base,database);
    for(const match of result.stdout.matchAll(/^WS8bm native browser logs: (.+)$/gm)) {
      for(const entry of JSON.parse(match[1])) if(/net::ERR_(?!ABORTED)/.test(entry.message)) (probe.networkFailures??=[]).push(entry.message);
    }
    const states=[...result.stdout.matchAll(/^WS8bm native mutation state: (.+)$/gm)].map(match=>JSON.parse(match[1]));
    const failures=[...result.stdout.matchAll(/^WS8bm native failures: (.+)$/gm)].flatMap(match=>JSON.parse(match[1]));
    probe.observed=states;probe.nativeFailures=failures;
    probe.ready=states.some(state=>['video','progress'].includes(label)?state.room==='/rooms/654632876'&&(label==='video'?state.videoJobPerformed:state.progressFault?.delivered):label==='release'?state.room==='/rooms/654632876'&&(state.releaseGeometry?.inMenu||state.releaseClicks?.some(click=>click.atPressPoint&&click.menuVisible)):state.room==='/rooms/201306877'&&state.open);
    assert.equal(result.status,0,`native pinned ${label} failed: ${result.error||result.signal||result.status}`);
  } finally {
    try {
      spawnSync('docker',['rm','-f',container],{stdio:'ignore'});
      if(proxy)await proxy.close();
    } finally {
      try {
        if(driver&&driver.exitCode===null) {const stopped=new Promise(resolve=>driver.once('exit',resolve));driver.kill('SIGTERM');await stopped;}
      } finally {
        try {if(log!==undefined) closeSync(log);}
        finally {rmSync(proof,{recursive:true,force:true});rmSync(temp,{recursive:true,force:true});}
      }
    }
  }
}

export function captureUploadReferenceLog(base,database) {
  // Test Rails logs normally live in its container. Preserve diagnostics
  // without changing assertion credit or preventing any caller's cleanup.
  try {
    const port=new URL(base).port;
    if(!database?.includes(`/.instances/${port}/`)) return;
    const root=fileURLToPath(new URL('../../../',import.meta.url));
    const scratch=root+'.scratch/ws8bm-native-phone';mkdirSync(scratch,{recursive:true});
    const contents=execFileSync('docker',['exec',`ws8bm-behavior-reference-${port}`,'cat','/rails/log/test.log'],{timeout:10000,maxBuffer:8*1024*1024});
    writeFileSync(scratch+`/attachment-${port}.log`,contents);
  }catch(error){console.error('WS8bm upload log diagnostic failed:',error.message);}
}
