// Rust-only ports of the pinned drive_attachments_test.rb, drive_link_previews_test.rb and
// sudo_mode_test.rb declarations (WS14g-224..230, 272). Each runs alone against its own
// app (drive_browser_tests.rs); the server-side Google API answers only the stubs below.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {Capybara,session,finish,users,rooms,connectGoogle,DRIVE_SCOPES,stubGoogleDriveList,stubGoogleDriveFile,driveFilePayload,lastMessage,createThread} from './drive-support.mjs';

const FILE_ID='1AbcDefGhIjKlMnOpQrSt';
const FILE_TWO='2BcdEfgHiJkLmNoPqRsTu';
const DOCS_URL=`https://docs.google.com/document/d/${FILE_ID}/edit`;
const PICKER='[role="dialog"][aria-label="Find a Drive file"]';
const CHIPS='.composer__drive-attachments .drive-attachment-chip';

// drive_attachments_test.rb / drive_link_previews_test.rb private helpers.
async function chooseDriveFromAttachMenu(c) {
  await (await c.find('button.composer__attachment-btn')).click();
  await c.clickButton('From Google Drive');
}
async function attachFromPicker(c,name) {
  const item=await c.find('.drive-picker__item',{text:name});
  await item.hover();
  await c.within(item,async()=>(await c.find('button.drive-picker__attach')).click());
}
async function withinChip(c,name,body) {return c.within(await c.find('.drive-attachment-chip',{text:name}),body);}
async function threadPanelSettled(c,at) {
  await c.synchronize(()=>c.evaluate("getComputedStyle(document.querySelector('#thread-panel .thread-panel__surface')).transform === 'none'"),{wait:10000,at});
}

test('WS14g-224 attach Drive files from the picker, send textless, and remove through edit',async t=>{
  const c=new Capybara(await session(t),'drive_attachments_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveList();
  await stubGoogleDriveFile(FILE_ID);
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);

  await chooseDriveFromAttachMenu(c);
  await c.assertSelector('.drive-picker__item',{text:'Q3 Planning',at:18});
  await c.within(PICKER,async()=>{
    await c.assertSelector('li[role="presentation"] > button[role="option"]',{at:21});
    await c.assertSelector('li[role="presentation"] > button.drive-picker__attach',{at:22});
    await c.assertNoSelector('[role="option"] button',{at:23});
  });

  await attachFromPicker(c,'Q3 Planning');
  await c.assertSelector(CHIPS,{text:'Q3 Planning',at:27});
  await c.assertNoSelector(PICKER,{at:28});

  await chooseDriveFromAttachMenu(c);
  await attachFromPicker(c,'Budget 2026');
  await c.assertSelector(CHIPS,{count:2,at:32});

  const size=await c.evaluate(`(() => {
  const button = document.querySelector(".drive-attachment-chip__remove");
  const rect = button.getBoundingClientRect();
  return { width: rect.width, height: rect.height };
})()`);
  assert.ok(size.width>=24,c.at(41)+': expected the Drive chip remove button to be at least 24px wide');
  assert.ok(size.height>=24,c.at(42)+': expected the Drive chip remove button to be at least 24px tall');

  // Dropping a pending chip keeps it out of the sent message.
  await withinChip(c,'Budget 2026',async()=>(await c.find('button')).click());
  await c.assertSelector(CHIPS,{count:1,at:46});

  await c.clickOn('Send Message');

  await c.assertNoSelector(CHIPS,{at:50});
  await c.assertSelector(`a.drive-attachment[href='https://drive.google.com/open?id=${FILE_ID}']`,{at:51});
  await c.assertSelector('.drive-attachments .drive-chip__name',{text:'Q3 Planning',at:52});
  const sent=await lastMessage();
  assert.deepEqual(sent.drive_file_ids,[FILE_ID],c.at(53));

  await c.visit(`/rooms/${rooms.designers}/messages/${sent.id}/edit`);
  await c.assertSelector('.drive-attachment-chip',{text:'Google Drive file',at:56});

  // The message is textless, so dropping its only attachment needs replacement text.
  await c.fillIn('Message','the file moved elsewhere');
  await c.clickOn('Remove Google Drive file');
  await c.assertNoSelector('.drive-attachment-chip',{at:61});

  await c.clickOn('Save changes');

  // The edit frame swaps the form for the rendered body; wait for the save first.
  await c.assertSelector('.message__body',{text:'the file moved elsewhere',wait:10000,at:73});
  await c.assertNoSelector('a.drive-attachment',{at:75});
  assert.deepEqual((await lastMessage()).drive_file_ids,[],c.at(76));
  await finish(c,'WS14g-224');
});

test('WS14g-225 edit a room message in the composer and remove one of two attachments',async t=>{
  const c=new Capybara(await session(t),'drive_attachments_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveList();
  await stubGoogleDriveFile(FILE_ID);
  await stubGoogleDriveFile(FILE_TWO,{body:driveFilePayload({name:'Budget 2026',mimeType:'application/vnd.google-apps.spreadsheet'})});
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);

  await chooseDriveFromAttachMenu(c);
  await attachFromPicker(c,'Q3 Planning');
  await chooseDriveFromAttachMenu(c);
  await attachFromPicker(c,'Budget 2026');
  await c.fillIn('Write a message','two files attached');
  await c.clickOn('Send Message');

  await c.assertSelector('a.drive-attachment',{count:2,at:94});
  await c.assertSelector('.drive-attachments .drive-chip__name',{text:'Q3 Planning',at:95});
  await c.assertSelector('.drive-attachments .drive-chip__name',{text:'Budget 2026',at:96});

  await c.withinMessage(await lastMessage(),()=>c.rightClickMessage());
  await c.cite(101,()=>c.assertMessageMenuOpen());
  await c.clickButton('Edit message');

  await c.assertSelector("[data-composer-target='contextLabel']",{text:'Editing Message',wait:10000,at:104});
  await c.assertSelector(CHIPS,{count:2,at:105});
  await withinChip(c,'Budget 2026',async()=>(await c.find('button')).click());
  await c.assertSelector(CHIPS,{count:1,at:107});
  await c.clickOn('Send Message');

  await c.assertSelector('a.drive-attachment',{count:1,at:110});
  await c.assertSelector(`a.drive-attachment[href='https://drive.google.com/open?id=${FILE_ID}']`,{at:111});
  await c.assertSelector("[data-composer-target='context'][hidden]",{visible:false,at:112});
  assert.deepEqual((await lastMessage()).drive_file_ids,[FILE_ID],c.at(113));
  await finish(c,'WS14g-225');
});

test('WS14g-226 attach a Drive file from the thread composer',async t=>{
  const c=new Capybara(await session(t),'drive_attachments_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveList();
  await stubGoogleDriveFile(FILE_ID);
  await c.signIn('jz@37signals.com');

  const thread=await createThread(rooms.designers,users.jz,'Drive thread');
  await c.joinRoom(rooms.designers);
  await c.visit(`/rooms/${rooms.designers}?thread=${thread}`);

  await c.assertSelector("#thread-panel [data-thread-panel-target='conversation']",{wait:10000,at:128});
  await c.within('#thread-panel',async()=>{
    await c.find('button.composer__attachment-btn',{wait:10000});
    await threadPanelSettled(c,159);
    await chooseDriveFromAttachMenu(c);
    await c.assertSelector(PICKER,{wait:10000,at:133});
    await c.assertSelector('.drive-picker__item',{text:'Q3 Planning',wait:10000,at:134});
    await attachFromPicker(c,'Q3 Planning');
    await c.assertSelector(CHIPS,{text:'Q3 Planning',at:136});
    await c.fillIn('Write a thread reply','thread file attached');
    await c.clickButton('Send Reply');
  });

  await c.assertSelector(`#thread-panel a.drive-attachment[href='https://drive.google.com/open?id=${FILE_ID}']`,{wait:10000,at:141});
  assert.deepEqual((await lastMessage(thread)).drive_file_ids,[FILE_ID],c.at(142));
  await finish(c,'WS14g-226');
});

test('WS14g-227 a viewer with the Drive scope sees a picked file upgraded to a preview chip',async t=>{
  const c=new Capybara(await session(t),'drive_link_previews_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveFile(FILE_ID);
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);

  await c.sendMessage(`Please review ${DOCS_URL} before Friday`);

  await c.assertSelector('.drive-chip__name',{text:'Q3 Planning',at:18});
  await c.assertSelector('.drive-chip__meta',{text:/Modified.+Riel/,at:19});
  await c.assertSelector('.drive-chip__icon svg',{at:20});
  await c.assertNoSelector('.drive-chip--plain',{at:21});
  await finish(c,'WS14g-227');
});

test('WS14g-228 a viewer with the Drive scope keeps a plain chip for a file never picked',async t=>{
  const c=new Capybara(await session(t),'drive_link_previews_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveFile(FILE_ID,{status:403,body:{}});
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);

  await c.sendMessage(`Please review ${DOCS_URL} before Friday`);

  await c.assertSelector('.drive-chip--plain .drive-chip__name',{text:'Google Doc',at:32});
  await c.assertNoSelector('.drive-chip__meta',{at:33});
  await finish(c,'WS14g-228');
});

test('WS14g-229 a viewer without the Drive scope sees a plain chip and fetches nothing',async t=>{
  const c=new Capybara(await session(t),'drive_link_previews_test.rb');
  await c.signIn('kevin@37signals.com');
  await c.joinRoom(rooms.designers);
  await installFetchRecorder(c);

  await c.sendMessage(`Please review ${DOCS_URL} before Friday`);

  await c.assertSelector('.drive-chip--plain .drive-chip__name',{text:'Google Doc',at:43});
  await c.assertNoSelector('.drive-chip__meta',{at:44});
  const driveRequests=await c.evaluate("window.driveFetchUrls.filter(url => url.includes('/google/drive/files'))");
  assert.deepEqual(driveRequests,[],c.at(48));
  await finish(c,'WS14g-229');
});

test('WS14g-230 composer Drive picker inserts the chosen file link at the caret',async t=>{
  const c=new Capybara(await session(t),'drive_link_previews_test.rb');
  await connectGoogle(users.jz,DRIVE_SCOPES);
  await stubGoogleDriveList();
  await c.signIn('jz@37signals.com');
  await c.joinRoom(rooms.designers);

  await c.fillInMarkdown('message_markdown_source','see brief');
  await c.execute(`const editor = document.getElementById("message_markdown_source")
editor.setSelectionRange(4, 4)
editor.focus()`);

  await chooseDriveFromAttachMenu(c);

  await c.assertSelector(PICKER,{at:66});
  await c.assertSelector('.drive-picker__item',{text:'Q3 Planning',at:67});

  await installDriveSearchRecorder(c);
  await c.fillIn('Search Drive files','plan');
  await c.synchronize(()=>c.evaluate(`window.settledDriveSearches.includes(${JSON.stringify('plan')})`),{wait:10000,at:110});

  await c.assertSelector('.drive-picker__item',{text:'Q3 Planning',at:73});
  await c.assertSelector('.drive-picker__meta',{text:/Modified.+Riel/,at:74});
  await c.assertSelector('.drive-picker__icon svg',{at:75});

  await (await c.findField('Search Drive files')).press('ArrowDown');

  await c.assertSelector('.drive-picker__item--active',{text:'Q3 Planning',at:79});

  await (await c.find('.drive-picker__item',{text:'Q3 Planning'})).click();

  assert.equal(await (await c.findField('message_markdown_source')).inputValue(),
    'see https://docs.google.com/document/d/1AbcDefGhIjKlMnOpQrSt/edit brief',c.at(83));
  await c.assertNoSelector(PICKER,{at:85});
  await finish(c,'WS14g-230');
});

// drive_link_previews_test.rb install_drive_search_recorder / install_fetch_recorder, verbatim.
async function installDriveSearchRecorder(c) {
  await c.execute(`window.settledDriveSearches = [];
if (!window.driveSearchWrapped) {
  window.driveSearchWrapped = true;
  const originalFetch = window.fetch.bind(window);
  window.fetch = (input, init) => {
    const url = String((input && input.url) || input);
    const promise = originalFetch(input, init);
    if (url.includes("/google/drive/files")) {
      const query = new URL(url, window.location.origin).searchParams.get("q");
      const record = () => window.settledDriveSearches.push(query);
      promise.then(record, record);
    }
    return promise;
  };
}`);
}
async function installFetchRecorder(c) {
  await c.execute(`window.driveFetchUrls = [];
if (!window.driveFetchWrapped) {
  window.driveFetchWrapped = true;
  const originalFetch = window.fetch.bind(window);
  window.fetch = (url, options) => {
    window.driveFetchUrls.push(String((url && url.url) || url));
    return originalFetch(url, options);
  };
}`);
}

test("WS14g-272 one prompt, then the action continues automatically",async t=>{
  const c=new Capybara(await session(t),'sudo_mode_test.rb');
  await c.signIn('david@37signals.com');
  await c.visit('/account/edit');
  const before=await (await c.findField('invite_url')).inputValue();
  assert.notEqual(before,'',c.at(11));

  await c.clickButton('Regenerate join link');

  // The sensitive action redirects to the sudo prompt.
  await c.assertSelector('h1',{text:"Confirm it's you",wait:10000,at:16});
  await c.fillIn('password','secret123456');
  await c.clickButton('Confirm');

  // Confirmation replays the stashed request without another click.
  await c.assertSelector('#invite_url',{wait:10000,at:21});
  const after=await (await c.findField('invite_url')).inputValue();
  assert.notEqual(after,'',c.at(23));
  assert.notEqual(after,before,c.at(24));
  await finish(c,'WS14g-272');
});
