import type { MessageDTO } from "../../gen/MessageDTO.ts";
import { permalinkTarget, type RoomAccess } from "./message-destination.ts";

type PermalinkMessage = Pick<MessageDTO, "id" | "roomId" | "threadId">;

interface FollowPermalink {
  readonly roomId: number;
  readonly messageId: number;
  readonly read: (messageId: number) => Promise<{ readonly message: PermalinkMessage }>;
  /** Membership that issued the read, or null while the room is still loading. */
  readonly access: () => RoomAccess | null;
  /**
   * The room detail has landed. A sidebar row can already say `member` while the join preview
   * is still up, and that must not count as the preview having joined.
   */
  readonly joined: () => boolean;
  readonly subscribe: (onChange: () => void) => () => void;
  readonly onFocus: (messageId: number) => void;
  readonly onRedirect: (href: string) => void;
  readonly onDrop: () => void;
}

/**
 * Reads a permalink and keeps it until the membership that sent the read can judge a miss.
 *
 * A failure uses the access captured when the read was issued. Join landing while that read
 * is in flight reads once more as a member; only a read issued as a member may drop the
 * anchor, and that retry cannot start another. An unjoined miss stays on the permalink until
 * `joined`, then reads once more without opening the room again. The returned function drops
 * a late result and releases the access subscription.
 */
export function followPermalink({
  roomId,
  messageId,
  read,
  access,
  joined,
  subscribe,
  onFocus,
  onRedirect,
  onDrop,
}: FollowPermalink): () => void {
  let live = true;

  let releaseWatch = (): void => {};

  const applyFound = (message: PermalinkMessage, openFound: boolean): void => {
    const target = permalinkTarget(roomId, messageId, "member", { status: "found", message });

    if (target.kind === "redirect") {
      onRedirect(target.href);

      return;
    }

    if (openFound) {
      onFocus(messageId);
    }
  };

  // After the preview joins, read again. That read is issued as a member: a miss drops
  // the anchor, and a root message is already the visit's focus.
  const followJoined = (): void => {
    const stop = subscribe(() => {
      if (!live || !joined()) {
        return;
      }

      stop();
      issueRead("member", false);
    });

    releaseWatch = stop;
  };

  const holdMissing = (membership: RoomAccess): void => {
    if (!live) {
      return;
    }

    const target = permalinkTarget(roomId, messageId, membership, { status: "missing" });

    if (target.kind === "focus" && target.messageId === null) {
      onDrop();

      return;
    }

    onFocus(messageId);
    followJoined();
  };

  // Access can still be loading when the read returns. Settle once, still using the
  // access that sent the read.
  const waitForAccess = (issued: RoomAccess | null): void => {
    let settled = false;

    const finish = (): void => {
      if (!live || settled || access() === null) {
        return;
      }

      settled = true;
      unsubscribe();
      settleFailed(issued);
    };

    const unsubscribe = subscribe(finish);

    releaseWatch = () => {
      settled = true;
      unsubscribe();
    };

    finish();
  };

  // Judge the 404 by the access that issued it. A join that lands while the read is in
  // flight used to look like a member miss and drop the permalink. Discard that result
  // and read once more as a member. The retry is issued as a member, so its own 404
  // drops the anchor and cannot start another read.
  const settleFailed = (issued: RoomAccess | null): void => {
    if (!live) {
      return;
    }

    const current = access();

    if (issued !== "member" && current === "member") {
      issueRead("member", true);

      return;
    }

    if (issued === "member") {
      holdMissing("member");

      return;
    }

    if (issued === "unjoined" || current === "unjoined") {
      holdMissing("unjoined");

      return;
    }

    waitForAccess(issued);
  };

  function issueRead(issued: RoomAccess | null, openFound: boolean): void {
    void read(messageId).then(
      ({ message }) => {
        if (!live) {
          return;
        }

        applyFound(message, openFound);
      },
      () => {
        settleFailed(issued);
      },
    );
  }

  issueRead(access(), true);

  return () => {
    live = false;
    releaseWatch();
  };
}
