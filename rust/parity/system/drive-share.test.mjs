// Rust-only ports of the 36 pinned drive_share_test.rb declarations (WS14g-232..267). Google
// Identity, the Picker and Drive REST are the Ruby test's own in-page doubles, copied verbatim
// into drive-share-mocks.js (MOCK_JS) and drive-share-lazy-mocks.js (LAZY_MOCK_JS); the
// server boots with the picker key and project number and fails any server-side Google call
// (drive_browser_tests.rs). Each declaration runs alone against its own app.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {test} from 'node:test';
import {Capybara,session,finish,control,users,rooms,lastMessage,createThread} from './drive-support.mjs';

const MOCK_JS=readFileSync(new URL('./drive-share-mocks.js',import.meta.url),'utf8');
const LAZY_MOCK_JS=readFileSync(new URL('./drive-share-lazy-mocks.js',import.meta.url),'utf8');
// test/support/drive_share_mocks.rb
const FILE_ID='1AbcDefGhIjKlMnOpQrSt';
const DRIVE_FILE_SCOPE='https://www.googleapis.com/auth/drive.file';
const DIALOG='.drive-share-dialog';
const PANEL='.drive-share__panel';
const RECIPIENT='.drive-share-dialog__recipient';
const SUMMARY='.drive-share-dialog__summary';
const CHIPS='.composer__drive-attachments .drive-attachment-chip';
const GRANT='Grant view access and attach';

function driveScenario(overrides={}) {
  return {
    token:{access_token:'mock-token-1',token_type:'Bearer',expires_in:3600,scope:DRIVE_FILE_SCOPE},
    picker:{id:FILE_ID,name:'Q3 Planning',mimeType:'application/vnd.google-apps.document'},
    file:{id:FILE_ID,name:'Q3 Planning',mimeType:'application/vnd.google-apps.document',capabilities:{canShare:true}},
    permissionPages:[[]],
    creates:{},
    failFirstListWith401:false,
    ...overrides,
  };
}
const injectDriveShareMocks=(c,scenario)=>c.execute(MOCK_JS,JSON.stringify(scenario));
const injectLazyMocks=(c,scenario)=>c.execute(LAZY_MOCK_JS,JSON.stringify(scenario));
const setMock=(c,path,value)=>c.execute(`window.__driveShareMock.scenario.${path} = ${JSON.stringify(value)}`);

// setup: sign in as JZ and join Designers.
async function setup(t) {
  const c=new Capybara(await session(t),'drive_share_test.rb');
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);
  return c;
}

// drive_share_test.rb private helpers; `line` is the calling line, cited with the helper's.
const mockCalls=(c,name)=>c.evaluate(`window.__driveShareMock.${name}`);
const mockCreateOrder=c=>c.evaluate('window.__driveShareMock.createOrder');
async function checkRecipient(c,name) {
  await c.within(await c.find(RECIPIENT,{text:name}),async()=>(await c.find('input')).click());
}
async function assertNoCheckedField(c,line) {
  const checked=await c.evaluate("document.querySelectorAll('.drive-share-dialog input:checked').length");
  c.verify(`${c.at(line)} (assert_no_checked_field :1029)`,()=>assert.equal(checked,0));
}
async function assertTokenHygiene(c,token,line) {
  const html=await c.evaluate('document.documentElement.outerHTML');
  c.verify(`${c.at(line)} (assert_token_hygiene :1035)`,()=>assert.ok(!html.includes(token),`expected the page HTML not to include ${token}`));
  const storage=await c.evaluate("[JSON.stringify({...localStorage}), JSON.stringify({...sessionStorage})].join(' ')");
  c.verify(`${c.at(line)} (assert_token_hygiene :1037)`,()=>assert.ok(!storage.includes(token),`expected storage not to include ${token}`));
}
async function assertDrivePanelHidden(c,line) {
  await c.assertSelector(`${PANEL}[hidden]`,{visible:false,at:`${c.at(line)} (assert_drive_panel_hidden :1041)`});
  await c.assertNoSelector(`${PANEL}:not([hidden])`,{at:`${c.at(line)} (assert_drive_panel_hidden :1042)`});
}
// Minitest::Assertion is not a StandardError, so Capybara's synchronize does not retry it:
// the focus check runs once.
async function assertAttachButtonFocused(c,line) {
  const focused=await c.evaluate("document.activeElement === document.querySelector('button.composer__attachment-btn')");
  c.verify(`${c.at(line)} (assert_attach_button_focused :1053)`,()=>assert.ok(focused,'expected focus to return to the attach button'));
}
async function waitForThreadPanelSettled(c,line) {
  await c.synchronize(()=>c.evaluate("getComputedStyle(document.querySelector('#thread-panel .thread-panel__surface')).transform === 'none'"),
    {wait:10000,at:`${c.at(line)} (wait_for_thread_panel_settled :1065): expected the thread panel slide transition to settle`});
}
async function chooseDriveFromAttachMenu(c,{wait}={}) {
  await (await c.find('button.composer__attachment-btn',wait?{wait}:{})).click();
  await c.clickButton('From Google Drive');
}
async function waitForDriveMock(c,name,count,line) {
  await c.synchronize(async()=>(await c.evaluate(`window.__driveShareMock.${name}.length`))>=count,
    {wait:10000,at:`${c.at(line)} (wait_for_drive_mock :1088): expected ${count} ${name}`});
}
const posts=calls=>calls.filter(call=>call.method==='POST');
const permissionCalls=calls=>calls.filter(call=>call.url.includes('/permissions'));
const fillStrip=`const strip = document.querySelector(".composer__drive-attachments");
for (let i = 0; i < 10; i++) {
  const chip = document.createElement("span");
  chip.className = "drive-attachment-chip";
  const input = document.createElement("input");
  input.type = "hidden";
  input.name = "message[drive_file_ids][]";
  input.value = \`prefilled\${i}1\`;
  chip.append(input);
  strip.append(chip);
}
`;
const emptyStrip="document.querySelector('.composer__drive-attachments').replaceChildren()";
const kevinOk="window.__driveShareMock.scenario.creates['kevin@37signals.com'] = 'ok'";

test('WS14g-232 review dialog offers attach-only and an explicit grant with names and emails',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);

  await c.within(DIALOG,async()=>{
    await c.assertSelector('.drive-share-dialog__file',{text:'Q3 Planning',at:121});
    await c.assertText('Access grants happen immediately',{at:122});
    await c.assertText('Future members are not added automatically',{at:123});
    await c.assertText('Only view access is granted',{at:124});
    await c.assertSelector(RECIPIENT,{count:3,at:125});
    await c.assertSelector(RECIPIENT,{text:'David',at:126});
    await c.assertSelector(RECIPIENT,{text:'david@37signals.com',at:127});
    await c.assertSelector(RECIPIENT,{text:'kevin@37signals.com',at:128});
    await c.assertSelector('.drive-share-dialog__select-all',{text:'Select all (3)',at:129});
    await assertNoCheckedField(c,130);
    await c.assertButton('Attach only',{at:131});
    await c.assertButton(GRANT,{disabled:true,at:132});
  });

  // The token client requested ONLY drive.file, without prior grants.
  const initCalls=await mockCalls(c,'initTokenCalls');
  c.verify(137,()=>assert.equal(initCalls.length,1));
  c.verify(138,()=>assert.equal(initCalls[0].scope,DRIVE_FILE_SCOPE));
  c.verify(139,()=>assert.equal(initCalls[0].include_granted_scopes,false));

  // The Picker was built with the project number, restricted key, OAuth token, and page origin.
  const build=(await mockCalls(c,'pickerBuilds'))[0];
  c.verify(144,()=>assert.equal(build.appId,'123456789012'));
  c.verify(145,()=>assert.equal(build.developerKey,'test-picker-key'));
  c.verify(146,()=>assert.equal(build.oauthToken,'mock-token-1'));
  const pageOrigin=await c.evaluate('window.location.origin');
  c.verify(147,()=>assert.equal(build.origin,pageOrigin));
  await finish(c,'WS14g-232');
});

test('WS14g-233 attach-only pins the chip and writes no Drive permissions',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,()=>c.clickOn('Attach only'));

  await c.assertNoSelector(DIALOG,{at:156});
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:157});
  await c.assertNoSelector('script[src*="google"]',{visible:false,at:158});

  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(161,()=>assert.equal(driveCalls.length,1,'expected only the files.get capability read'));
  c.verify(162,()=>assert.match(driveCalls[0].url,new RegExp(`/drive/v3/files/${FILE_ID}\\?`)));
  c.verify(163,()=>assert.ok(driveCalls[0].url.includes('capabilities')));
  c.verify(164,()=>assert.equal(driveCalls[0].auth,'Bearer mock-token-1'));
  c.verify(165,()=>assert.deepEqual(permissionCalls(driveCalls),[]));

  // No recipient validation happens on the attach-only path either.
  const recipientsCalls=await mockCalls(c,'recipientsCalls');
  c.verify(168,()=>assert.deepEqual(posts(recipientsCalls),[]));

  await c.clickOn('Send Message');

  await c.assertSelector(`a.drive-attachment[href='https://drive.google.com/open?id=${FILE_ID}']`,{at:172});
  const sent=await lastMessage();
  c.verify(173,()=>assert.deepEqual(sent.drive_file_ids,[FILE_ID]));
  await c.assertNoSelector(CHIPS,{at:174});
  await assertTokenHygiene(c,'mock-token-1',175);
  await finish(c,'WS14g-233');
});

test('WS14g-234 grant validates, preserves writers, and creates only missing readers',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({permissionPages:[[
    {id:'perm-david',type:'user',role:'writer',emailAddress:'david@37signals.com'},
    {id:'perm-kevin',type:'user',role:'reader',emailAddress:'kevin@37signals.com'},
  ]]}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await (await c.find('.drive-share-dialog__select-all input')).click();
    await c.assertButton(GRANT,{disabled:false,at:191});
    await c.clickOn(GRANT);

    await c.assertSelector(SUMMARY,{exactText:'All 3 recipients have access: 1 newly granted; 2 already had access. The file is attached below.',wait:10000,at:194});
    await c.assertSelector('.drive-share-dialog__result--already',{text:'already had access',count:2,at:195});
    await c.assertSelector('.drive-share-dialog__result--granted',{text:'jason@37signals.com',at:196});
    await c.clickOn('Done');
  });

  // The selection was re-validated against current membership first.
  const validations=posts(await mockCalls(c,'recipientsCalls'));
  c.verify(202,()=>assert.equal(validations.length,1));
  c.verify(203,()=>assert.ok(validations[0].url.includes('/drive_recipients/validate')));

  // Exactly one sequential reader grant, silent, for the missing member.
  const creates=posts(await mockCalls(c,'driveCalls'));
  const order=await mockCreateOrder(c);
  c.verify(207,()=>assert.deepEqual(order,['jason@37signals.com']));
  c.verify(208,()=>assert.equal(creates.length,1));
  c.verify(209,()=>assert.ok(creates[0].url.includes('sendNotificationEmail=false')));
  c.verify(210,()=>assert.ok(creates[0].url.includes('supportsAllDrives=true')));
  c.verify(211,()=>assert.deepEqual(JSON.parse(creates[0].body),{role:'reader',type:'user',emailAddress:'jason@37signals.com'}));

  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:216});
  await assertTokenHygiene(c,'mock-token-1',217);
  await finish(c,'WS14g-234');
});

test('WS14g-235 partial failure reports per recipient and retries only outstanding grants',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({creates:{'kevin@37signals.com':{status:403,reason:'domainPolicy'}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);

    await c.assertSelector(SUMMARY,{text:'granted to 1 of 2',wait:10000,at:231});
    await c.assertSelector('.drive-share-dialog__result--granted',{text:'jason@37signals.com',at:232});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'kevin@37signals.com',at:233});
    await c.assertText('organization sharing policy',{at:234});
  });

  // The file is attached with honest status, never reported as success.
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:238});

  await c.execute(kevinOk);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{text:'View access granted to 2 recipients',wait:10000,at:243});
    await c.assertNoSelector('.drive-share-dialog__result--failed',{at:244});
    await c.clickOn('Done');
  });

  // Jason granted once; only Kevin retried, after a fresh permissions re-read converged the
  // ambiguous result.
  const order=await mockCreateOrder(c);
  c.verify(250,()=>assert.deepEqual(order,['jason@37signals.com','kevin@37signals.com','kevin@37signals.com']));
  const lists=(await mockCalls(c,'driveCalls')).filter(call=>call.method==='GET'&&call.url.includes('/permissions'));
  c.verify(252,()=>assert.ok(lists.length>=2,'expected permissions re-read before retry'));
  await c.assertSelector(CHIPS,{count:1,at:253});
  await finish(c,'WS14g-235');
});

test('WS14g-236 cancelled picker selection shares nothing and can be retried',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({picker:'cancel'}));

  await chooseDriveFromAttachMenu(c);
  await waitForDriveMock(c,'pickerVisible',1,260);

  await assertDrivePanelHidden(c,262);
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(263,()=>assert.deepEqual(driveCalls,[]));

  await setMock(c,'picker',driveScenario().picker);
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:268});
  await c.within(DIALOG,()=>c.clickOn('Attach only'));
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:270});
  await finish(c,'WS14g-236');
});

test('WS14g-237 cancelled Google authorization shares nothing and can be retried',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{error:'access_denied'}}));

  await chooseDriveFromAttachMenu(c);
  await waitForDriveMock(c,'requestTokenCalls',1,277);

  await assertDrivePanelHidden(c,279);
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(280,()=>assert.deepEqual(driveCalls,[]));
  const builds=await mockCalls(c,'pickerBuilds');
  c.verify(281,()=>assert.deepEqual(builds,[]));

  await setMock(c,'token',driveScenario().token);
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:286});
  await finish(c,'WS14g-237');
});

test('WS14g-238 picker cancel returns quietly and the drive button works again',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({picker:'cancel'}));

  await chooseDriveFromAttachMenu(c);
  await waitForDriveMock(c,'pickerVisible',1,293);

  await assertDrivePanelHidden(c,295);
  await assertAttachButtonFocused(c,296);
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(297,()=>assert.deepEqual(driveCalls,[]));

  await setMock(c,'picker',driveScenario().picker);
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:302});
  await c.within(DIALOG,()=>c.clickOn('Attach only'));
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:304});
  await finish(c,'WS14g-238');
});

test('WS14g-239 closed consent popup returns quietly to the composer',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{error:'popup_closed'}}));

  await chooseDriveFromAttachMenu(c);
  await waitForDriveMock(c,'requestTokenCalls',1,311);

  await assertDrivePanelHidden(c,313);
  await assertAttachButtonFocused(c,314);
  const builds=await mockCalls(c,'pickerBuilds');
  c.verify(315,()=>assert.deepEqual(builds,[]));
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(316,()=>assert.deepEqual(driveCalls,[]));

  await setMock(c,'token',driveScenario().token);
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:321});
  await finish(c,'WS14g-239');
});

test('WS14g-240 consent popup closed via GIS error callback returns quietly',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,{...driveScenario(),tokenErrorCallback:{type:'popup_closed'}});

  await chooseDriveFromAttachMenu(c);
  await waitForDriveMock(c,'requestTokenCalls',1,328);

  await assertDrivePanelHidden(c,330);
  await assertAttachButtonFocused(c,331);
  const builds=await mockCalls(c,'pickerBuilds');
  c.verify(332,()=>assert.deepEqual(builds,[]));

  await c.execute('window.__driveShareMock.scenario.tokenErrorCallback = null');
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:337});
  await finish(c,'WS14g-240');
});

test('WS14g-241 real error dialog closes with the Close button and the drive button works again',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{error:'server_error'}}));

  await chooseDriveFromAttachMenu(c);

  await c.within(PANEL,async()=>{
    await c.assertText('Google authorization failed. Nothing was shared.',{wait:10000,at:346});
    await c.assertButton('Try again',{at:347});
    await c.assertButton('Close',{at:348});
  });

  await c.within(PANEL,()=>c.clickOn('Close'));

  await assertDrivePanelHidden(c,353);
  await assertAttachButtonFocused(c,354);

  await setMock(c,'token',driveScenario().token);
  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:359});
  await finish(c,'WS14g-241');
});

test('WS14g-242 real error dialog closes with Esc',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{error:'server_error'}}));

  await chooseDriveFromAttachMenu(c);

  await c.within(PANEL,async()=>{
    await c.assertText('Google authorization failed. Nothing was shared.',{wait:10000,at:368});
  });

  await (await c.find(`${PANEL} .drive-share__action`)).press('Escape');

  await assertDrivePanelHidden(c,373);
  await assertAttachButtonFocused(c,374);
  await finish(c,'WS14g-242');
});

test('WS14g-243 try again re-opens the picker after a real error',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{error:'server_error'}}));

  await chooseDriveFromAttachMenu(c);

  await c.within(PANEL,async()=>{
    await c.assertText('Google authorization failed. Nothing was shared.',{wait:10000,at:383});
    await c.assertButton('Close',{at:384});
    await c.assertButton('Try again',{at:385});
  });
  const buildsBefore=(await mockCalls(c,'pickerBuilds')).length;

  await setMock(c,'token',driveScenario().token);
  await c.within(PANEL,()=>c.clickOn('Try again'));

  await c.assertSelector(DIALOG,{visible:true,wait:10000,at:392});
  const buildsAfter=(await mockCalls(c,'pickerBuilds')).length;
  c.verify(393,()=>assert.ok(buildsAfter>buildsBefore,`expected ${buildsAfter} to be > ${buildsBefore}`));
  await finish(c,'WS14g-243');
});

test('WS14g-244 script load failure offers retry without hanging the composer',async t=>{
  const c=await setup(t);
  await injectLazyMocks(c,driveScenario({failPickerLoad:true}));

  await chooseDriveFromAttachMenu(c);

  await c.within(PANEL,async()=>{
    await c.assertText('Google Drive could not be reached.',{wait:10000,at:402});
    await c.assertButton('Try again',{at:403});
  });

  await injectDriveShareMocks(c,driveScenario());
  await c.within(PANEL,()=>c.clickOn('Try again'));

  await c.assertSelector(DIALOG,{visible:true,at:409});
  await finish(c,'WS14g-244');
});

test('WS14g-245 first use loads scripts then continues on a fresh gesture',async t=>{
  const c=await setup(t);
  await injectLazyMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);

  await c.within(PANEL,async()=>{
    await c.assertButton('Continue with Google',{wait:10000,at:418});
    await c.clickOn('Continue with Google');
  });

  await c.assertSelector(DIALOG,{visible:true,at:422});
  const tokenCalls=await mockCalls(c,'requestTokenCalls');
  c.verify(423,()=>assert.equal(tokenCalls.length,1));
  await finish(c,'WS14g-245');
});

test('WS14g-246 unshareable file disables the grant but keeps attach-only',async t=>{
  const c=await setup(t);
  const file={...driveScenario().file,capabilities:{canShare:false}};
  await injectDriveShareMocks(c,driveScenario({file}));

  await chooseDriveFromAttachMenu(c);

  await c.within(DIALOG,async()=>{
    await c.assertText('do not have permission to share this file',{at:433});
    await c.assertText("organization's Drive policies",{at:434});
    await c.assertButton(GRANT,{disabled:true,at:435});
    await c.clickOn('Attach only');
  });

  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:439});
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(440,()=>assert.deepEqual(permissionCalls(driveCalls),[]));
  await finish(c,'WS14g-246');
});

test('WS14g-247 folder selection disables the grant but keeps attach-only',async t=>{
  const c=await setup(t);
  const picker={id:FILE_ID,name:'Team folder',mimeType:'application/vnd.google-apps.folder'};
  const file={...picker,capabilities:{canShare:true}};
  await injectDriveShareMocks(c,driveScenario({picker,file}));

  await chooseDriveFromAttachMenu(c);

  await c.within(DIALOG,async()=>{
    await c.assertSelector('.drive-share-dialog__file',{text:'Team folder',at:451});
    await c.assertText('Folders and shortcuts cannot be shared from here',{at:452});
    await c.assertButton(GRANT,{disabled:true,at:453});
    await c.clickOn('Attach only');
  });

  await c.assertSelector(CHIPS,{text:'Team folder',at:457});
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(458,()=>assert.deepEqual(permissionCalls(driveCalls),[]));
  await finish(c,'WS14g-247');
});

test('WS14g-248 expired Google session reconnects on an explicit gesture and continues',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({failFirstListWith401:true}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await c.clickOn(GRANT);
    await c.assertButton('Reconnect Google Drive',{wait:10000,at:468});
  });

  await setMock(c,'token',{...driveScenario().token,access_token:'mock-token-2'});
  await c.within(DIALOG,async()=>{
    await c.clickOn('Reconnect Google Drive');
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:474});
    await c.clickOn('Done');
  });

  const creates=posts(await mockCalls(c,'driveCalls'));
  c.verify(479,()=>assert.equal(creates.length,1));
  c.verify(480,()=>assert.equal(creates[0].auth,'Bearer mock-token-2'));
  await assertTokenHygiene(c,'mock-token-2',481);
  await finish(c,'WS14g-248');
});

test('WS14g-249 grant rejects a recipient who left mid-review and refreshes the list',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await c.assertSelector(RECIPIENT,{text:'Kevin',at:489});
  });

  await control('/__drive_browser__/memberships/revoke',{room:rooms.designers,user:users.kevin});

  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Kevin');
    await checkRecipient(c,'David');
    await c.clickOn(GRANT);
    await c.assertText('no longer in this chat',{wait:10000,at:498});
    await c.assertNoSelector(RECIPIENT,{text:'Kevin',at:499});
    await c.assertSelector(RECIPIENT,{text:'David',at:500});
    await c.clickOn('Attach only');
  });

  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:504});
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(505,()=>assert.deepEqual(posts(driveCalls),[]));
  await finish(c,'WS14g-249');
});

test('WS14g-250 thread composer grants against the parent room membership',async t=>{
  const c=await setup(t);
  const thread=await createThread(rooms.designers,users.jz,'Drive thread');
  await c.joinRoom(rooms.designers);
  await c.visit(`/rooms/${rooms.designers}?thread=${thread}`);
  await c.assertSelector("#thread-panel [data-thread-panel-target='conversation']",{visible:true,wait:10000,at:514});

  await injectDriveShareMocks(c,driveScenario());

  await c.within('#thread-panel',async()=>{
    await waitForThreadPanelSettled(c,519);
    await chooseDriveFromAttachMenu(c,{wait:10000});
  });

  await c.within(DIALOG,async()=>{
    await c.assertSelector('.drive-share-dialog__file',{text:'Q3 Planning',wait:10000,at:524});
    await c.assertSelector(RECIPIENT,{text:'david@37signals.com',at:525});
    await checkRecipient(c,'David');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:528});
    await c.clickOn('Done');
  });

  await c.within('#thread-panel',async()=>{
    await c.assertSelector(CHIPS,{text:'Q3 Planning',at:533});
    await c.fillIn('Write a thread reply','thread file attached');
    await c.clickButton('Send Reply');
  });

  await c.assertSelector(`#thread-panel a.drive-attachment[href='https://drive.google.com/open?id=${FILE_ID}']`,{wait:10000,at:538});
  const reply=await lastMessage(thread);
  c.verify(539,()=>assert.deepEqual(reply.drive_file_ids,[FILE_ID]));
  await finish(c,'WS14g-250');
});

test('WS14g-251 navigation disposes the dialog, token, and picker state',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.assertSelector(DIALOG,{visible:true,at:546});

  await c.execute("document.dispatchEvent(new Event('turbo:before-cache'))");

  await c.assertNoSelector(DIALOG,{wait:5000,at:550});
  await c.assertNoSelector(`${PANEL}:not([hidden])`,{at:551});
  await assertTokenHygiene(c,'mock-token-1',552);

  await c.within('#sidebar',async()=>{
    await c.assertSelector('a',{text:'HQ',at:555});
    await c.clickLink('HQ');
  });
  await c.assertSelector('.room-header__name',{text:'HQ',wait:10000,at:558});
  await c.assertNoSelector(DIALOG,{at:559});
  await assertTokenHygiene(c,'mock-token-1',560);
  await finish(c,'WS14g-251');
});

test('WS14g-252 mobile viewport keeps the review dialog usable',async t=>{
  const c=await setup(t);
  await c.page.setViewportSize({width:390,height:844});
  try {
    await injectDriveShareMocks(c,driveScenario());

    await chooseDriveFromAttachMenu(c);

    await c.assertSelector(DIALOG,{visible:true,at:570});
    await c.assertSelector(RECIPIENT,{visible:true,at:571});
    const geometry=await c.evaluate(`(() => {
  const dialog = document.querySelector(".drive-share-dialog");
  const rect = dialog.getBoundingClientRect();
  const row = dialog.querySelector(".drive-share-dialog__recipient").getBoundingClientRect();
  return {
    left: rect.left, right: rect.right, width: rect.width,
    viewport: window.innerWidth, rowHeight: row.height
  };
})()
`);
    c.verify(583,()=>assert.ok(geometry.left>=0,`expected left ${geometry.left} to be >= 0`));
    c.verify(584,()=>assert.ok(geometry.right<=geometry.viewport,`expected right ${geometry.right} to be <= ${geometry.viewport}`));
    c.verify(585,()=>assert.ok(geometry.rowHeight>=44,`expected row height ${geometry.rowHeight} to be >= 44`));

    await c.within(DIALOG,()=>c.clickOn('Attach only'));
    await c.assertSelector(CHIPS,{text:'Q3 Planning',at:588});
  } finally {
    await c.page.setViewportSize({width:1400,height:1400});
  }
  await finish(c,'WS14g-252');
});

test('WS14g-253 grant requires fresh review when a recipient email changes',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await c.assertSelector(RECIPIENT,{text:'kevin@37signals.com',at:599});
  });

  await control('/__drive_browser__/users/email',{user:users.kevin,email_address:'kevin.new@37signals.com'});

  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertText('Recipient details changed since this review',{wait:10000,at:607});
    await c.assertSelector(RECIPIENT,{text:'kevin.new@37signals.com',at:608});
    await assertNoCheckedField(c,609);
    await c.clickOn('Attach only');
  });

  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:613});
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(614,()=>assert.deepEqual(posts(driveCalls),[]));
  await finish(c,'WS14g-253');
});

test('WS14g-254 changed identity can be re-approved against the new email',async t=>{
  const c=await setup(t);
  await control('/__drive_browser__/users/email',{user:users.kevin,email_address:'kevin.new@37signals.com'});
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:625});
    await c.clickOn('Done');
  });

  const creates=posts(await mockCalls(c,'driveCalls'));
  const order=await mockCreateOrder(c);
  c.verify(630,()=>assert.deepEqual(order,['kevin.new@37signals.com']));
  c.verify(631,()=>assert.equal(creates.length,1));
  await finish(c,'WS14g-254');
});

test('WS14g-255 retry revalidates membership before writing',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({creates:{'kevin@37signals.com':{status:403,reason:'domainPolicy'}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'granted to 1 of 2',wait:10000,at:644});
  });

  await control('/__drive_browser__/memberships/revoke',{room:rooms.designers,user:users.kevin});

  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertText('no longer in this chat',{wait:10000,at:651});
    await c.assertNoSelector(RECIPIENT,{text:'Kevin',at:652});
    await c.assertSelector(RECIPIENT,{text:'David',at:653});
    await checkRecipient(c,'David');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:656});
    await c.clickOn('Done');
  });

  const order=await mockCreateOrder(c);
  c.verify(660,()=>assert.deepEqual(order,['jason@37signals.com','kevin@37signals.com','david@37signals.com']));
  await finish(c,'WS14g-255');
});

test('WS14g-256 reconnect revalidates before resuming grants',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({failFirstListWith401:true}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertButton('Reconnect Google Drive',{wait:10000,at:671});
  });

  await control('/__drive_browser__/memberships/revoke',{room:rooms.designers,user:users.kevin});
  await setMock(c,'token',{...driveScenario().token,access_token:'mock-token-2'});

  await c.within(DIALOG,async()=>{
    await c.clickOn('Reconnect Google Drive');
    await c.assertText('no longer in this chat',{wait:10000,at:679});
    await c.assertNoSelector(RECIPIENT,{text:'Kevin',at:680});
    await c.assertSelector(RECIPIENT,{text:'David',at:681});
  });

  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(684,()=>assert.deepEqual(posts(driveCalls),[]));
  await finish(c,'WS14g-256');
});

test('WS14g-257 cancelled authorization invalidates a delayed token callback',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({token:{access_token:'delayed-token'}}));
  await c.execute(`const mock = window.__driveShareMock;
const oauth = window.google.accounts.oauth2;
const init = oauth.initTokenClient;
oauth.initTokenClient = (config) => {
  const client = init(config);
  return { requestAccessToken: (override) => {
    mock.requestTokenCalls.push(override || {});
  }};
};
`);

  await chooseDriveFromAttachMenu(c);
  await c.within(PANEL,async()=>{
    await c.assertText('Waiting for Google authorization',{at:703});
    await c.clickOn('Cancel');
  });
  await assertDrivePanelHidden(c,706);
  await assertAttachButtonFocused(c,707);

  await c.execute("window.__driveShareMock.lastTokenConfig.callback({access_token: 'delayed-token'})");

  await c.assertNoSelector(DIALOG,{at:711});
  const builds=await mockCalls(c,'pickerBuilds');
  c.verify(712,()=>assert.deepEqual(builds,[]));
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(713,()=>assert.deepEqual(driveCalls,[]));
  await finish(c,'WS14g-257');
});

test('WS14g-258 cancelled picker invalidates a delayed selection callback',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({picker:'manual'}));

  await chooseDriveFromAttachMenu(c);
  await c.within(PANEL,async()=>{
    await c.assertText('Choose a file in the Google Drive window',{wait:10000,at:721});
    await c.clickOn('Cancel');
  });
  await assertDrivePanelHidden(c,724);
  await assertAttachButtonFocused(c,725);

  await c.execute(`window.__driveShareMock.lastPickerCallback({action: 'picked', docs: [${JSON.stringify(driveScenario().picker)}]})`);

  await c.assertNoSelector(DIALOG,{at:731});
  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(732,()=>assert.deepEqual(driveCalls,[]));
  await finish(c,'WS14g-258');
});

test('WS14g-259 grant is blocked when attachments are already full',async t=>{
  const c=await setup(t);
  await c.execute(fillStrip);
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'David');
    await c.clickOn(GRANT);
    await c.assertText('remove one to grant and attach',{wait:10000,at:755});
  });

  const driveCalls=await mockCalls(c,'driveCalls');
  c.verify(758,()=>assert.deepEqual(permissionCalls(driveCalls),[]));

  await c.execute(emptyStrip);
  await c.within(DIALOG,async()=>{
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:763});
    await c.clickOn('Done');
  });

  const order=await mockCreateOrder(c);
  c.verify(767,()=>assert.deepEqual(order,['david@37signals.com']));
  await finish(c,'WS14g-259');
});

test('WS14g-260 mid-flight capacity loss preserves grant outcomes',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({fillStripOnCreate:true}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'David');
    await checkRecipient(c,'Jason');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'not attached yet',wait:10000,at:778});
    await c.assertSelector('.drive-share-dialog__result--granted',{count:2,at:779});
    await c.assertText('Grant results are shown below and are unaffected',{at:780});
    await c.assertButton('Attach file',{at:781});
  });

  await c.assertNoSelector(CHIPS,{text:'Q3 Planning',at:784});

  await c.execute(emptyStrip);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Attach file');
    await c.assertSelector(SUMMARY,{text:'The file is attached below',at:789});
    await c.clickOn('Done');
  });

  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:793});
  await finish(c,'WS14g-260');
});

test('WS14g-261 rate-limited grants are classified and retry cleanly',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({creates:{'kevin@37signals.com':{status:429}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'granted to 1 of 2',wait:10000,at:806});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'rate limited',at:807});
    await c.assertText('rate-limited these grants',{at:808});
    await c.assertNoText('Google denied every grant',{at:809});
  });

  await c.execute(kevinOk);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{text:'View access granted to 2 recipients',wait:10000,at:815});
    await c.clickOn('Done');
  });

  const order=await mockCreateOrder(c);
  c.verify(819,()=>assert.deepEqual(order,['jason@37signals.com','kevin@37signals.com','kevin@37signals.com']));
  await finish(c,'WS14g-261');
});

test('WS14g-262 dialog checkboxes are visible and long names wrap',async t=>{
  const c=await setup(t);
  await control('/__drive_browser__/users',{name:'Alexandria Montgomery-Beauregard the Third of Accounting',
    email_address:'alexandria.montgomery-beauregard.the.third@37signals.com',password:'secret123456',room:rooms.designers});
  await injectDriveShareMocks(c,driveScenario());

  await chooseDriveFromAttachMenu(c);
  await c.assertSelector(RECIPIENT,{text:'Alexandria Montgomery-Beauregard',wait:10000,at:828});

  const styles=await c.evaluate(`(() => {
  const box = document.querySelector(".drive-share-dialog__recipient input");
  const name = document.querySelector(".drive-share-dialog__recipient-name");
  const email = document.querySelector(".drive-share-dialog__recipient-email");
  const boxRect = box.getBoundingClientRect();
  const boxStyle = getComputedStyle(box);
  return {
    width: boxRect.width, height: boxRect.height,
    appearance: boxStyle.appearance,
    nameWrap: getComputedStyle(name).overflowWrap,
    nameWhiteSpace: getComputedStyle(name).whiteSpace,
    emailWrap: getComputedStyle(email).overflowWrap
  };
})()
`);
  c.verify(846,()=>assert.ok(styles.width>0,`expected width ${styles.width} to be > 0`));
  c.verify(847,()=>assert.ok(styles.height>0,`expected height ${styles.height} to be > 0`));
  c.verify(848,()=>assert.notEqual(styles.appearance,'none'));
  c.verify(849,()=>assert.equal(styles.nameWrap,'anywhere'));
  c.verify(850,()=>assert.equal(styles.nameWhiteSpace,'normal'));
  c.verify(851,()=>assert.equal(styles.emailWrap,'anywhere'));
  await finish(c,'WS14g-262');
});

test('WS14g-263 retry does not write while attachment capacity is unavailable',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({fillStripOnCreate:true,creates:{'kevin@37signals.com':{status:403,reason:'domainPolicy'}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'granted to 1 of 2',wait:10000,at:865});
    await c.assertSelector(SUMMARY,{text:'not attached yet',at:866});
    await c.assertButton('Attach file',{at:867});
  });
  const firstOrder=await mockCreateOrder(c);
  c.verify(869,()=>assert.deepEqual(firstOrder,['jason@37signals.com','kevin@37signals.com']));
  await c.assertNoSelector(CHIPS,{text:'Q3 Planning',at:870});

  await c.execute(kevinOk);

  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertText('remove one to grant and attach',{wait:10000,at:876});
    await c.assertSelector('.drive-share-dialog__result--granted',{text:'jason@37signals.com',at:877});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'kevin@37signals.com',at:878});
  });

  const blockedOrder=await mockCreateOrder(c);
  c.verify(881,()=>assert.deepEqual(blockedOrder,['jason@37signals.com','kevin@37signals.com']));

  await c.execute(emptyStrip);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{text:'View access granted to 2 recipients',wait:10000,at:886});
    await c.assertSelector(SUMMARY,{text:'The file is attached below',at:887});
    await c.clickOn('Done');
  });

  const finalOrder=await mockCreateOrder(c);
  c.verify(891,()=>assert.deepEqual(finalOrder,['jason@37signals.com','kevin@37signals.com','kevin@37signals.com']));
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:892});
  await finish(c,'WS14g-263');
});

test('WS14g-264 completed outcomes stay visible through reconnect and refreshed review',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({create401OnAttempt:2}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'David');
    await checkRecipient(c,'Jason');
    await c.clickOn(GRANT);
    await c.assertButton('Reconnect Google Drive',{wait:10000,at:903});
    await c.within('.drive-share-dialog__completed',async()=>{
      await c.assertText('persist in Drive even if you cancel',{at:905});
      await c.assertSelector('.drive-share-dialog__result--granted',{text:'david@37signals.com',at:906});
    });
    await c.assertNoSelector('.drive-share-dialog__completed .drive-share-dialog__result',{text:'jason@37signals.com',at:908});
  });

  await control('/__drive_browser__/memberships/revoke',{room:rooms.designers,user:users.jason});
  await setMock(c,'token',{...driveScenario().token,access_token:'mock-token-2'});

  await c.within(DIALOG,async()=>{
    await c.clickOn('Reconnect Google Drive');
    await c.assertText('no longer in this chat',{wait:10000,at:916});
    await c.within('.drive-share-dialog__completed',async()=>{
      await c.assertSelector('.drive-share-dialog__result--granted',{text:'david@37signals.com',at:918});
    });
    await c.assertSelector(RECIPIENT,{text:'Kevin',at:920});
    await c.clickOn('Cancel');
  });

  await c.assertNoSelector(DIALOG,{at:924});
  const order=await mockCreateOrder(c);
  c.verify(925,()=>assert.deepEqual(order,['david@37signals.com','jason@37signals.com']));
  await finish(c,'WS14g-264');
});

test('WS14g-265 reconciled access is confirmed, not claimed as newly granted',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({creates:{'kevin@37signals.com':{status:500}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'granted to 1 of 2',wait:10000,at:938});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'service error',at:939});
  });

  await c.execute(`window.__driveShareMock.scenario.permissionPages = [
  [ { id: "perm-kevin", type: "user", role: "reader", emailAddress: "kevin@37signals.com" } ]
];
`);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{exactText:'All 2 recipients have access: 1 newly granted; 1 confirmed in Drive. The file is attached below.',wait:10000,at:949});
    await c.assertSelector('.drive-share-dialog__result--confirmed',{text:'kevin@37signals.com',at:950});
    await c.assertSelector('.drive-share-dialog__result--confirmed',{text:'access confirmed',at:951});
    await c.clickOn('Done');
  });

  const order=await mockCreateOrder(c);
  c.verify(955,()=>assert.deepEqual(order,['jason@37signals.com','kevin@37signals.com']));
  await finish(c,'WS14g-265');
});

test('WS14g-266 existing and reconciled access are distinguished with no new grants',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({
    permissionPages:[[{id:'perm-jason',type:'user',role:'writer',emailAddress:'jason@37signals.com'}]],
    creates:{'kevin@37signals.com':{status:500}},
  }));
  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Jason');
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector('.drive-share-dialog__result--already',{text:'jason@37signals.com',at:968});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'kevin@37signals.com',at:969});
  });
  await c.execute(`window.__driveShareMock.scenario.permissionPages = [[
  { id: "perm-jason", type: "user", role: "writer", emailAddress: "jason@37signals.com" },
  { id: "perm-kevin", type: "user", role: "owner", emailAddress: "kevin@37signals.com" }
]];
`);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{exactText:'All 2 recipients have access: 1 already had access; 1 confirmed in Drive. The file is attached below.',wait:10000,at:979});
    await c.assertNoSelector('.drive-share-dialog__result--granted',{at:980});
    await c.clickOn('Done');
  });
  const order=await mockCreateOrder(c);
  c.verify(983,()=>assert.deepEqual(order,['kevin@37signals.com']));
  await finish(c,'WS14g-266');
});

test('WS14g-267 server errors are classified as service failures, not policy denials',async t=>{
  const c=await setup(t);
  await injectDriveShareMocks(c,driveScenario({creates:{'kevin@37signals.com':{status:503}}}));

  await chooseDriveFromAttachMenu(c);
  await c.within(DIALOG,async()=>{
    await checkRecipient(c,'Kevin');
    await c.clickOn(GRANT);
    await c.assertSelector(SUMMARY,{text:'No access was granted',wait:10000,at:995});
    await c.assertSelector('.drive-share-dialog__result--failed',{text:'service error',at:996});
    await c.assertText('service error',{at:997});
    await c.assertNoText('refused by Google',{at:998});
    await c.assertNoText('Google denied every grant',{at:999});
    await c.assertNoText('sharing policy',{at:1000});
  });

  await c.execute(kevinOk);
  await c.within(DIALOG,async()=>{
    await c.clickOn('Retry 1 remaining');
    await c.assertSelector(SUMMARY,{text:'View access granted',wait:10000,at:1006});
    await c.clickOn('Done');
  });

  const order=await mockCreateOrder(c);
  c.verify(1010,()=>assert.deepEqual(order,['kevin@37signals.com','kevin@37signals.com']));
  await finish(c,'WS14g-267');
});
