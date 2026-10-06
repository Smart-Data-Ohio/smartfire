/**
 * A UUID version 7 (RFC 9562): 48 bits of Unix time in milliseconds, then random bits, so ids
 * made later sort later. `now` is passed in so callers can use their own clock.
 */
export function uuid7(now: number): string {
  const bytes = new Uint8Array(16);

  crypto.getRandomValues(bytes);

  let time = Math.max(0, Math.floor(now));

  for (let index = 5; index >= 0; index -= 1) {
    bytes[index] = time % 256;
    time = Math.floor(time / 256);
  }

  bytes[6] = 0x70 | ((bytes[6] ?? 0) & 0x0f);
  bytes[8] = 0x80 | ((bytes[8] ?? 0) & 0x3f);

  const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");

  return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
}
