import { test } from "@playwright/test";

/**
 * S1 item 8: the realtime scenarios, against the Rust binary with the SPA embedded and a frozen
 * seed (two signed-in browser contexts). Skipped until CI can boot that server; each test lists
 * the steps it will take so the harness slice only has to fill them in.
 */
test.describe
  .skip("realtime (needs the Rust server with the SPA embedded)", () => {
    test("two people in a room see each other's messages live", async () => {
      // 1. Sign in as Alice in one context and Bob in another; both open /app/r/<general>.
      // 2. Alice types: Bob sees "Alice is typing…" under his composer within a second.
      // 3. Alice sends "hello from Alice": her row shows at once at 60 % opacity (pending), then
      //    settles to full opacity when the POST returns; Bob's timeline gains the same row once,
      //    and his typing line clears.
      // 4. Bob replies; Alice sees it rise in at the bottom without scrolling.
    });

    test("reconnect after a server restart resumes without duplicates", async () => {
      // 1. Alice and Bob in the same room; Alice notes the last message id.
      // 2. Restart the server process (same database). Alice's banner shows "Reconnecting…"
      //    after the grace period, then "Back online".
      // 3. While Alice was disconnected Bob sent two messages: Alice's timeline holds each exactly
      //    once (resume replays from the cursor, or `resumed: false` triggers a resync of the room).
      // 4. A message Alice sent while offline goes out after the reconnect, once.
    });

    test("the unread badge clears on read", async () => {
      // 1. Alice views #design; Bob posts in #general and @mentions Alice.
      // 2. Alice's sidebar row for #general turns bold with a mention pill of 1; the Home rail
      //    item gets its badge and the tab title shows "(1)".
      // 3. Alice opens #general: the unread divider sits above Bob's message, and once she is at
      //    the bottom the row, the pill, the rail badge and the title count clear (POST /read).
      // 4. Reloading keeps it read.
    });
  });
