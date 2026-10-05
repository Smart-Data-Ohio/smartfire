// Execute both new declarations with the pin's own Selenium/Capybara body.
import assert from 'node:assert/strict';
import {nativePhone} from './behavior-native-phone.mjs';
export const VIDEO_CASE='uploading a fresh video in the thread composer';
export const PROGRESS_CASE='late upload progress preserves a delivered attachment and reply preview';
export const uploadCases=[VIDEO_CASE,PROGRESS_CASE];
export function extractUpload(source, name) {
  const declaration=`  test "${name}" do\n`,start=source.indexOf(declaration),end=source.indexOf('\n  test ',start+1);
  assert.ok(start>=0&&end>start,'pinned upload declaration boundaries');
  return {body:source.slice(start,end).replace(`test "${name}" do`,'def test_phone'),helpers:''};
}
export async function nativeUpload(base,database,name,probe={},mutated=false) {
  const video=name===VIDEO_CASE;
  const sourcePath=`test/system/${video?'sending_messages':'workspace_markdown'}_test.rb`;
  // The first mutant prevents the real processed presentation from gaining
  // its poster. The second restores the pre-#231 delivered-row overwrite.
  const mutation=video
    ?['models/client_message-','import { escapeHTML } from "helpers/string_helpers"',`import { escapeHTML } from "helpers/string_helpers"\nwindow.__ws8bmVideoFault=true; new MutationObserver(() => { document.querySelectorAll('#thread-panel video[poster]').forEach(video=>{window.__ws8bmVideoFaultSeen=true; video.removeAttribute('poster')}) }).observe(document,{subtree:true,childList:true,attributes:true,attributeFilter:['poster']});`]
    :['models/client_message-','return element?.hasAttribute("data-message-id") ? null : element','window.__ws8bmProgressFault = { delivered: element?.hasAttribute("data-message-id"), id: clientMessageId }; return element'];
  await nativePhone(base,{sourcePath,extract:source=>extractUpload(source,name),label:video?'video':'progress',database,probe,...(mutated?{mutation}:{})});
}
