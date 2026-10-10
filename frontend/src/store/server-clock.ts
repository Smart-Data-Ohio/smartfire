export interface ServerClock {
  readonly serverAt: number;
  readonly receivedAt: number;
}

/** Unknown server time keeps deadlines active until a server sample arrives. */
export function serverNow(clock: ServerClock | undefined): number {
  return clock === undefined ? 0 : clock.serverAt + performance.now() - clock.receivedAt;
}

export function sampleServerClock(evaluatedAt: string, held: ServerClock | undefined): ServerClock {
  return {
    serverAt: Math.max(Date.parse(evaluatedAt), serverNow(held)),
    receivedAt: performance.now(),
  };
}
