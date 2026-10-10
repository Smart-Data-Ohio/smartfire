import type { MessageSound } from "../src/gen/MessageSound.ts";

/** The classic sounds exercised by the mock workspace. Production reads the Rust catalog. */
export function mockSound(markdown: string): MessageSound | null {
  switch (markdown) {
    case "/play bell":
      return { name: "bell", url: "/assets/bell.mp3", presentation: { kind: "text", text: "🔔" } };
    case "/play 56k":
      return {
        name: "56k",
        url: "/assets/56k.mp3",
        presentation: { kind: "image", url: "/assets/sounds/56k.webp", width: 79, height: 33 },
      };
    default:
      return null;
  }
}
