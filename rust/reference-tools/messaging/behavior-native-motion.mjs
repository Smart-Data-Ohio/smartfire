// Execute the original motion body/helpers without a diagnostic sidebar wait.
import assert from 'node:assert/strict';
import {nativePhone} from './behavior-native-phone.mjs';
export const NATIVE_MOTION_CASE='mobile drawer animates in, lands in place, and returns focus with motion on';
export function extractMotion(source) {
  const declaration=`  test "${NATIVE_MOTION_CASE}" do\n`;
  const start=source.indexOf(declaration),end=source.indexOf('\n  test ',start+1);
  const helpers=source.indexOf('\n  def motion_token(');
  assert.ok(start>=0&&end>start&&helpers>end,'pinned motion source boundaries');
  return {body:source.slice(start,end).replace(`test "${NATIVE_MOTION_CASE}" do`,'def test_phone'),helpers:source.slice(helpers,source.lastIndexOf('\nend'))};
}
export async function nativeMotion(base) {
  await nativePhone(base,{sourcePath:'test/system/motion_test.rb',extract:extractMotion,line:26,label:'motion'});
}
