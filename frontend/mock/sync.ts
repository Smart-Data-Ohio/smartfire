/**
 * The mock's `/api/v1/sync` hub: connections, topics, sequence numbers, the replay ring for
 * resume, typing fan-out and keep-alive pings. Transport-free: the Vite plugin bridges it to real
 * WebSockets and the Vitest stub to an in-process fake.
 */
import type { ClientFrame } from "../src/gen/ClientFrame.ts";
import type { ServerFrame } from "../src/gen/ServerFrame.ts";
import type { SyncEvent } from "../src/gen/SyncEvent.ts";
import type { SyncPayload } from "../src/gen/SyncPayload.ts";
import {
  booleanField,
  field,
  intField,
  isRecord,
  type Json,
  stringArrayField,
  stringField,
} from "./json.ts";
import type { Scheduler } from "./scheduler.ts";

/** How many recent events are kept for resume. */
export const RING_SIZE = 500;

/** A `ping` goes out after this long without any other frame. */
export const PING_AFTER_MS = 15_000;

/** One socket's handle on the hub: frames in, and the way to hang up. */
export interface SyncConnection {
  receive(frame: ClientFrame): void;
  /** The transport closed: forget the connection. */
  close(): void;
}

/** Sends a frame to the client. */
export type SendFrame = (frame: ServerFrame) => void;

/** Closes the socket from the server side without a goodbye (the transport's abrupt close). */
export type DropSocket = () => void;

/** An event to publish: where it goes and what it says. */
export type Outgoing = { readonly topic: string } & SyncPayload;

interface Connection {
  readonly id: number;
  readonly send: SendFrame;
  readonly drop: DropSocket;
  readonly topics: Set<string>;
  readonly present: Set<number>;
  pingTimer: number | null;
  open: boolean;
}

export interface SyncHub {
  connect(send: SendFrame, drop?: DropSocket): SyncConnection;
  /** Gives each event the next sequence number, logs it and sends it to its subscribers. */
  publish(events: readonly Outgoing[]): void;
  /** Rooms some connection says it is looking at (`present`). */
  presentRooms(): ReadonlySet<number>;
  /** Rooms some connection is subscribed to. */
  subscribedRooms(): number[];
  /** Hangs up every connection, abruptly or after a `bye{reconnect:true}`. */
  dropAll(bye: boolean): void;
  /** Tells every connection to refetch its topics. */
  resyncAll(reason: string): void;
  /** Starts over as a new server boot: a new epoch, an empty log, every connection dropped. */
  restart(epoch: string): void;
  epoch(): string;
  seq(): number;
  connectionCount(): number;
}

export interface SyncHubOptions {
  readonly scheduler: Scheduler;
  readonly epoch: string;
  readonly viewerId: number;
}

export function createSyncHub(options: SyncHubOptions): SyncHub {
  const { scheduler, viewerId } = options;
  const connections = new Set<Connection>();
  let epoch = options.epoch;
  let seq = 0;
  let ring: SyncEvent[] = [];
  let nextConnectionId = 1;

  const armPing = (connection: Connection) => {
    if (connection.pingTimer !== null) scheduler.cancel(connection.pingTimer);
    connection.pingTimer = scheduler.schedule(PING_AFTER_MS, () => {
      connection.pingTimer = null;
      transmit(connection, { t: "ping" });
    });
  };

  const transmit = (connection: Connection, frame: ServerFrame) => {
    if (!connection.open) return;
    connection.send(frame);
    armPing(connection);
  };

  const forget = (connection: Connection) => {
    connection.open = false;

    if (connection.pingTimer !== null) scheduler.cancel(connection.pingTimer);

    connection.pingTimer = null;
    connections.delete(connection);
  };

  const record = (outgoing: Outgoing): SyncEvent => {
    seq += 1;

    const event: SyncEvent = { seq, ...outgoing };

    ring.push(event);

    if (ring.length > RING_SIZE) ring = ring.slice(ring.length - RING_SIZE);

    return event;
  };

  const deliver = (events: readonly SyncEvent[], except: Connection | null) => {
    for (const connection of connections) {
      if (connection === except) continue;

      const mine = events.filter((event) => connection.topics.has(event.topic));

      if (mine.length > 0) transmit(connection, { t: "batch", events: mine });
    }
  };

  const hello = (connection: Connection, frame: Extract<ClientFrame, { t: "hello" }>) => {
    connection.topics.clear();
    connection.topics.add("user");

    for (const topic of frame.topics) connection.topics.add(topic);

    const resume = frame.resume;
    const oldest = ring[0]?.seq ?? seq + 1;

    const canResume =
      resume !== null && resume.epoch === epoch && resume.seq <= seq && resume.seq >= oldest - 1;

    if (!canResume) {
      transmit(connection, { t: "welcome", epoch, seq, resumed: false });

      return;
    }

    // Welcome at the client's own point, then replay what it missed, so a client that takes
    // its cursor from `welcome` still applies the replay.
    transmit(connection, { t: "welcome", epoch, seq: resume.seq, resumed: true });

    const missed = ring.filter(
      (event) =>
        event.seq > resume.seq && event.type !== "typing" && connection.topics.has(event.topic),
    );

    if (missed.length > 0) transmit(connection, { t: "batch", events: missed });
  };

  const dropAll = (bye: boolean) => {
    for (const connection of [...connections]) {
      if (bye) transmit(connection, { t: "bye", reconnect: true, reason: "server_restart" });
      forget(connection);
      connection.drop();
    }
  };

  const receive = (connection: Connection, frame: ClientFrame) => {
    if (!connection.open) return;

    switch (frame.t) {
      case "hello":
        hello(connection, frame);
        break;
      case "sub":
        for (const topic of frame.topics) connection.topics.add(topic);
        break;
      case "unsub":
        for (const topic of frame.topics) {
          if (topic !== "user") connection.topics.delete(topic);
        }

        break;
      case "typing":
        deliver(
          [record({ topic: frame.conv, type: "typing", data: { userId: viewerId, on: frame.on } })],
          connection,
        );
        break;
      case "present":
        connection.present.add(frame.room);
        break;
      case "absent":
        connection.present.delete(frame.room);
        break;
      case "hb":
        break;
    }
  };

  return {
    connect(send, drop = () => {}) {
      const connection: Connection = {
        id: nextConnectionId++,
        send,
        drop,
        topics: new Set(["user"]),
        present: new Set(),
        pingTimer: null,
        open: true,
      };

      connections.add(connection);
      armPing(connection);

      return {
        receive: (frame) => receive(connection, frame),
        close: () => forget(connection),
      };
    },
    publish(events) {
      deliver(events.map(record), null);
    },
    presentRooms() {
      const rooms = new Set<number>();

      for (const connection of connections) {
        for (const room of connection.present) rooms.add(room);
      }

      return rooms;
    },
    subscribedRooms() {
      const rooms = new Set<number>();

      for (const connection of connections) {
        for (const topic of connection.topics) {
          const match = /^room:(\d+)$/.exec(topic);

          if (match !== null) rooms.add(Number(match[1]));
        }
      }

      return [...rooms].sort((a, b) => a - b);
    },
    dropAll,
    resyncAll(reason) {
      for (const connection of connections) {
        transmit(connection, { t: "resync", topics: [...connection.topics].sort(), reason });
      }
    },
    restart(nextEpoch) {
      dropAll(true);
      epoch = nextEpoch;
      seq = 0;
      ring = [];
    },
    epoch: () => epoch,
    seq: () => seq,
    connectionCount: () => connections.size,
  };
}

/** Parses a client frame off the wire; `null` for anything that isn't one. */
export function parseClientFrame(json: Json | undefined): ClientFrame | null {
  if (!isRecord(json)) return null;

  const topics = stringArrayField(json, "topics");

  switch (stringField(json, "t")) {
    case "hello": {
      const v = intField(json, "v");
      const resume = field(json, "resume");
      const epoch = stringField(resume, "epoch");
      const seq = intField(resume, "seq");

      if (v === null || topics === null) return null;

      if (resume !== null && resume !== undefined && (epoch === null || seq === null)) return null;

      return {
        t: "hello",
        v,
        resume: epoch === null || seq === null ? null : { epoch, seq },
        topics,
      };
    }

    case "sub":
      return topics === null ? null : { t: "sub", topics };
    case "unsub":
      return topics === null ? null : { t: "unsub", topics };
    case "typing": {
      const conv = stringField(json, "conv");
      const on = booleanField(json, "on");

      return conv === null || on === null ? null : { t: "typing", conv, on };
    }

    case "present":
    case "absent": {
      const room = intField(json, "room");
      const t = stringField(json, "t") === "present" ? "present" : "absent";

      return room === null ? null : { t, room };
    }

    case "hb": {
      const active = booleanField(json, "active");

      return active === null ? null : { t: "hb", active };
    }

    default:
      return null;
  }
}
