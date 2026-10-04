import assert from 'node:assert/strict';
import {test} from 'node:test';
import {execFileSync} from 'node:child_process';
import {PIN} from './reference-pin.mjs';
import {uploadCases,extractUpload} from './behavior-native-uploads.mjs';
import {rejectionEvidence} from './behavior-discrimination.mjs';
for(const [index,name] of uploadCases.entries()) {
  const path=`test/system/${index===0?'sending_messages':'workspace_markdown'}_test.rb`;
  const source=execFileSync('git',['show',`${PIN}:${path}`],{encoding:'utf8'});
  test(`${name}: exact original body, visibility scopes, deadlines and browser writes`,()=>{
    const {body,helpers}=extractUpload(source,name);
    const start=source.indexOf(`  test "${name}" do\n`),end=source.indexOf('\n  test ',start+1);
    assert.equal(body.replace('def test_phone',`test "${name}" do`),source.slice(start,end));
    assert.equal(helpers,'');
  });
  test(`${name}: only intended assertion and encountered fault earn rejection credit`,()=>{
    const line=index===0?53:215;
    const state=index===0?{videoJobPerformed:true,videoFaultSeen:true}:{progressFault:{delivered:true}};
    const error={code:'ERR_ASSERTION',stack:'AssertionError: native pinned upload failed'};
    const probe={ready:true,applied:1,observed:[state],nativeFailures:[{assertion:true,message:'intended fault',backtrace:[`${path}:${line}:in test_phone`]}]};
    assert.equal(rejectionEvidence(name,'default',probe,error).valid,true);
    assert.equal(rejectionEvidence(name,'default',{...probe,observed:[]},error).valid,false);
    assert.equal(rejectionEvidence(name,'default',{...probe,nativeFailures:[{assertion:true,message:'earlier setup',backtrace:[`${path}:${line-1}:in test_phone`]}]},error).valid,false);
    assert.equal(rejectionEvidence(name,'default',{...probe,networkFailures:['network error']},error).valid,false);
  });
}
