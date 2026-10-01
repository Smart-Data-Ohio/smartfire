// Source: test/system/unread_divider_test.rb at d7c7de92; scroll behaviour only.
import assert from 'node:assert/strict';
export async function unreadDivider({author:page,base,caseName,fixture}) {
  const list=page.locator('.messages[role="log"]').first(),divider=page.locator('#unread-divider'),pill=page.locator('#jump-to-unread');
  async function above(id) {
    await divider.waitFor();
    assert.equal(await divider.evaluate(node=>node.nextElementSibling?.dataset.messageId),String(id));
  }
  if(caseName.startsWith('mark unread')) {
    const target=page.locator(`.message[data-message-id="${fixture.target_id}"]`);
    await target.locator('[data-message-edit-format], [data-reply-target="body"]').first().click({button:'right'});
    await page.getByRole('menuitem',{name:'Mark unread',exact:true}).click();
    await page.locator('#sidebar .unread').filter({hasText:'Designers'}).waitFor();
    assert.equal((await page.goto(base+'/rooms/654632876')).status(),200);
    await above(fixture.target_id);
  } else if(caseName.startsWith('unread older')) {
    assert.equal(await list.locator('.message').filter({hasText:'First unread off page'}).count(),0);
    await pill.waitFor();const href=await pill.getAttribute('href');
    assert.equal(new URL(href,base).searchParams.get('message_id'),String(fixture.first_unread_id));
    // The original test re-marks the pointer after presence reads the room.
    // Exercise the public endpoint for the same state instead of editing DB.
    const status=await page.evaluate(async id=>{
      const response=await fetch(`/rooms/654632876/read?message_id=${id}`,{method:'DELETE',headers:{Accept:'application/json','X-CSRF-Token':document.querySelector('meta[name="csrf-token"]').content}});
      const body=await response.json();return {status:response.status,body};
    },fixture.first_unread_id);
    assert.equal(status.status,200);assert.equal(status.body.first_unread_message_id,fixture.first_unread_id);
    await pill.click();await above(fixture.first_unread_id);
    await list.locator('.message').filter({hasText:'First unread off page'}).waitFor();
  } else {
    await above(fixture.first_unread_id);
    if(caseName.startsWith('few unread')) {
      assert.match(await divider.textContent(),/new messages/i);
      const distance=await list.evaluate(node=>node.scrollHeight-node.scrollTop-node.clientHeight);
      assert.ok(distance<100,`same pinned bottom-scroll bound: ${distance}`);
    } else if(caseName.startsWith('many unread')) {
      const geometry=await divider.evaluate(node=>{
        const list=node.closest('.messages'),a=list.getBoundingClientRect(),b=node.getBoundingClientRect();
        return {top:b.top-a.top,height:a.height};
      });
      assert.ok(geometry.top<geometry.height/2,JSON.stringify(geometry));
    } else if(caseName.startsWith('the jump pill')) {
      await list.evaluate(node=>{node.scrollTop=node.scrollHeight;});await pill.waitFor();
      assert.match(await pill.textContent(),/Jump to unread/);
      await pill.click();
      await pill.waitFor({state:'hidden'});await above(fixture.first_unread_id);
    } else throw new Error(`unimplemented unread case ${caseName}`);
  }
}
