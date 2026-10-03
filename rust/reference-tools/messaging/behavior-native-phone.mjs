// Positive phone parity uses the actual pinned Capybara/Selenium sequence.
// No added focus assertion or diagnostic wait earns declaration credit.
import assert from 'node:assert/strict';
import {spawn,spawnSync,execFileSync} from 'node:child_process';
import {mkdirSync,writeFileSync,readFileSync,mkdtempSync,rmSync,openSync,closeSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import {dirname,basename} from 'node:path';
import {createServer} from 'node:net';
import {createHash} from 'node:crypto';
export const PHONE_CASE='keeps the thread drawer usable on a phone and preserves the channel';
const PIN='d7c7de9264c63015be398001d7a1094e7695a6db';
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
export async function nativePhone(base,{sourcePath='test/system/threads_test.rb',extract=extractPhone,line=293,label='phone',database}={}) {
  const root=fileURLToPath(new URL('../../../',import.meta.url));
  const tools=fileURLToPath(new URL('./',import.meta.url));
  const scratch=root+'.scratch/ws8bm-native-phone';
  mkdirSync(scratch,{recursive:true});
  const proof=mkdtempSync(scratch+'/proof-');
  // Chrome's Unix socket path must be short. This owned cache is authorized;
  // no /tmp path or another worker's files/listeners are used.
  const cache='/home/riels/.cache/rust-port/ws8bm';mkdirSync(cache,{recursive:true});
  const temp=mkdtempSync(cache+'/s-');
  const container='ws8bm-native-'+basename(temp);
  let driver,log;
  try {
    const source=execFileSync('git',['show',`${PIN}:${sourcePath}`],{cwd:root,encoding:'utf8'});
    const {body,helpers}=extract(source);
    writeFileSync(proof+'/phone-body.rb',body);writeFileSync(proof+'/phone-helpers.rb',helpers);
    writeFileSync(proof+'/native-location.json',JSON.stringify({sourcePath,line,label}));
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
    console.log(`WS8bm native ${label} source: Rails ${PIN}; SHA256 ${createHash('sha256').update(source).digest('hex')}; ${label==='attachment'?'original behavior body; shared file path adapted':'unchanged behavior body/helpers, screenshots omitted'}`);
    const extra=database?['--env-file',root+'rust/parity/.env.reference','-e','RAILS_ENV=test','-e','WS8BM_NATIVE_DATABASE=/readback/'+basename(database),'-e',`WS8BM_NATIVE_UPLOAD=${temp}/markdown-workspace-attachment.txt`,'-v',`${dirname(database)}:/readback:ro`,'-v',`${temp}:${temp}`]:[];
    if(database&&process.env.CI!==undefined) extra.push('-e',`CI=${process.env.CI}`);
    const result=spawnSync('docker',['run','--rm','--name',container,'--network','host','--cpus','2',...extra,'-v',`${proof}:/proof:ro`,'-v',`${tools}:/tools:ro`,'-e',`WS8BM_NATIVE_BASE=${base}`,'--entrypoint','bundle',process.env.PARITY_IMAGE||'triage-reference-d7c7de92','exec','ruby','/tools/behavior-native-phone.rb'],{encoding:'utf8',timeout:120000,maxBuffer:8*1024*1024});
    process.stdout.write(result.stdout||'');process.stderr.write(result.stderr||'');
    if(label==='attachment') captureUploadReferenceLog(base,database);
    assert.equal(result.status,0,`native pinned ${label} failed: ${result.error||result.signal||result.status}`);
  } finally {
    // Even a Ruby diagnostic exception or subprocess timeout cannot retain
    // the native container/driver. This container is uniquely owned here.
    try {execFileSync('docker',['rm','-f',container],{stdio:'ignore'});}catch{}
    if(driver&&driver.exitCode===null) {const stopped=new Promise(resolve=>driver.once('exit',resolve));driver.kill('SIGTERM');await stopped;}
    if(log!==undefined) closeSync(log);
    rmSync(proof,{recursive:true,force:true});rmSync(temp,{recursive:true,force:true});
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
