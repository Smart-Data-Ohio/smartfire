// Rust-only port of the pinned events_test.rb declaration "scheduling an event announces it in
// the room with a card members respond from" (WS14e-101). The HTTP tests cover its forms, card
// and response (rooms/events/tests/cutover/interactions.rs); only a browser can show that the
// Going click is answered inside the card's Turbo frame and the page stays on the room.
import assert from 'node:assert/strict';
import {test} from 'node:test';
import {Capybara,session,finish,users,rooms,control} from './drive-support.mjs';

// 8.days.from_now.strftime("%Y-%m-%dT15:30") at the app's frozen clock, 2026-03-02 16:00 UTC.
const STARTS='2026-03-10T15:30';

test('WS14e-101 scheduling an event announces it in the room with a card members respond from',async t=>{
  const david=new Capybara(await session(t),'events_test.rb');
  await david.signIn('david@37signals.com');
  await david.joinRoom(rooms.designers);
  await david.clickOn('Events');
  await david.clickOn('New event');
  await david.fillIn('Title','Card session');
  await david.fillIn('Starts',STARTS);
  await david.clickOn('Schedule event');
  await david.assertSelector('h1',{text:'Card session',at:115});

  const jason=new Capybara(await session(t),'events_test.rb');
  await jason.signIn('jason@37signals.com');
  await jason.joinRoom(rooms.designers);
  await jason.assertText('Scheduled an event: Card session',{at:124});
  await jason.within(await jason.find('.event-card',{text:'Card session'}),async()=>{
    await jason.assertText('Organized by David',{at:127});
    await jason.assertText('No response yet',{at:128});
    await jason.clickButton('Going');
    await jason.assertText('Currently: Going',{at:132});
  });

  // The response landed without leaving the room.
  await jason.synchronize(async()=>{
    const url=new URL(jason.page.url());
    return url.pathname+url.search===`/rooms/${rooms.designers}`;
  },{at:136});
  await jason.assertText('Scheduled an event: Card session',{at:137});

  const {response}=await control(`/__drive_browser__/events/response?title=${encodeURIComponent('Card session')}&user=${users.jason}`);
  jason.verify(140,()=>assert.equal(response,'going'));
  await finish(jason,'WS14e-101');
});
