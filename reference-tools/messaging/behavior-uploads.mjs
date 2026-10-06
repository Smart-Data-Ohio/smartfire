// sending_messages_test.rb's fresh thread video and workspace_markdown_test.rb's late
// upload progress, with the pinned upload files. The Python readback checks the saved
// rows, blob bytes, preview image and webp variant (sending_messages_test.rb:46-52).
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {waitForVisibility,waitForVisibleCount,actOnVisible,filterVisibleText} from './behavior-visibility.mjs';
import {BROADCAST_WAIT} from './behavior-deadlines.mjs';
import {assertMenuOpen,originalMessage} from './behavior-actions.mjs';
export const VIDEO_CASE='uploading a fresh video in the thread composer';
export const PROGRESS_CASE='late upload progress preserves a delivered attachment and reply preview';
export const uploadCases=[VIDEO_CASE,PROGRESS_CASE];
const upload=name=>fileURLToPath(new URL(`../../test-support/behavior-sources/test/fixtures/files/${name}`,import.meta.url));
const waitForCable=page=>page.waitForFunction(()=>{
  const sources=[...document.querySelectorAll('turbo-cable-stream-source')];
  return sources.length>=3&&sources.every(source=>source.hasAttribute('connected'));
},null,{timeout:BROADCAST_WAIT});

export async function uploads({author:page,base,caseName,fixture,probe}) {
  if(caseName===VIDEO_CASE) await video(page,base,fixture,probe);
  else await lateProgress(page,base);
}

async function video(page,base,fixture,probe) {
  const response=await page.goto(`${base}/rooms/654632876?thread=${fixture.video_thread_id}`);
  assert.equal(response.status(),200);
  await waitForCable(page);
  const panel=page.locator('#thread-panel');
  await waitForVisibility(panel.getByRole('combobox',{name:'Write a thread reply',exact:true}),{timeout:BROADCAST_WAIT});
  await panel.locator('input[type=file]').setInputFiles(upload('alpha-centuri.mov'));
  await actOnVisible(panel.getByRole('button',{name:'Send Reply',exact:true}),'click',{});
  await waitForVisibility(panel.locator('video.message__attachment'),{timeout:BROADCAST_WAIT});
  // perform_enqueued_jobs(only: Message::AttachmentProcessingJob): the test host runs
  // exactly the queued processing jobs; every other queue stays stopped.
  const performed=await page.request.post(`${base}/__ws8bm__/attachment-processing`);
  assert.equal(performed.status(),200,'the queued attachment processing job ran');
  probe.videoJobPerformed=(await performed.json()).performed>0;
  await waitForVisibility(page.locator("#thread-panel div[style*='aspect-ratio'] video.message__attachment[poster]"),{timeout:BROADCAST_WAIT});
}

// Message.joins(:attachment_attachment).find_by!(reply_to_message_id: parent.id), by its DOM id.
function attachedReplyDomId(database) {
  const id=execFileSync('python3',['-c',`import sqlite3,sys
c=sqlite3.connect("file:"+sys.argv[1]+"?mode=ro",uri=True)
row=c.execute("SELECT m.client_message_id FROM messages m JOIN active_storage_attachments a ON a.record_type='Message' AND a.name='attachment' AND a.record_id=m.id WHERE m.reply_to_message_id=607264868 LIMIT 1").fetchone()
assert row, "a saved reply with an attachment"
print(row[0])`,database],{encoding:'utf8'}).trim();
  return 'message_'+id;
}

async function lateProgress(page,base) {
  await actOnVisible(originalMessage(page).locator('[data-message-edit-format], [data-reply-target="body"]').first(),'click',{button:'right'});
  await assertMenuOpen(page);
  await actOnVisible(page.getByRole('menuitem',{name:'Reply',exact:true}),'click',{});
  await waitForVisibility(filterVisibleText(page.locator('#composer [data-composer-target="contextLabel"]'),'Replying to JZ'));
  // Hold the real uploader and its progress callback independently, so the
  // callback can arrive after both the POST and the delivered DOM replacement.
  const held=await page.evaluate(()=>import('models/file_uploader').then(({default:FileUploader})=>{
    const upload=FileUploader.prototype.upload;
    FileUploader.prototype.upload=function() {
      const progress=this.progressCallback;
      this.progressCallback=()=>{};
      return new Promise(resolve=>{
        window.attachmentUpload={
          progress:percent=>progress(percent,this.clientMessageId,this.file),
          resume:()=>resolve(upload.call(this)),
        };
      });
    };
    return true;
  }).catch(error=>error.message));
  assert.equal(held,true);
  await page.locator("#composer input[type='file']").setInputFiles(upload('moon.jpg'));
  await actOnVisible(page.getByRole('button',{name:'Send Message',exact:true}),'click',{});
  const pending=page.locator('.message:not([data-message-id]) .message__pending-upload');
  await waitForVisibility(filterVisibleText(pending,'moon.jpg - 0%'));
  await page.evaluate(()=>window.attachmentUpload.progress(50));
  await waitForVisibility(filterVisibleText(pending,'moon.jpg - 50%'));
  await page.evaluate(()=>window.attachmentUpload.resume());
  await waitForVisibility(filterVisibleText(page.locator('.message[data-message-id] .message__reply-preview'),"Third time's a charm."),{timeout:BROADCAST_WAIT});
  const reply=page.locator(`[id=${JSON.stringify(attachedReplyDomId(JSON.parse(process.env.WS8BM_WORK_DATABASES)[base]))}]`);
  await waitForVisibility(reply.locator('img.message__attachment'),{timeout:BROADCAST_WAIT});
  const body=()=>reply.locator('.message__body-content').evaluate(node=>node.innerHTML);
  const delivered=await body();
  await page.evaluate(()=>window.attachmentUpload.progress(100));
  assert.equal(await body(),delivered,'progress: delivered body unchanged');
  await waitForVisibility(filterVisibleText(reply.locator('.message__reply-preview'),"Third time's a charm."));
  await waitForVisibility(reply.locator('img.message__attachment'));
  await waitForVisibleCount(reply.locator('.message__pending-upload'),0);
}
