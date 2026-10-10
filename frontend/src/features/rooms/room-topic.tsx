import type { ReactNode } from "react";

/** Topic text stays literal; only web URLs become links. */
export function RoomTopic({ topic }: { readonly topic: string }) {
  const parts: ReactNode[] = [];
  let end = 0;

  for (const match of topic.matchAll(/(?:https?:\/\/|www\.)[^\s<>"']+/g)) {
    const start = match.index;
    let label = match[0].replace(/[.,!?;:]+$/, "");

    while (label.endsWith(")") && label.split(")").length > label.split("(").length) {
      label = label.slice(0, -1);
    }

    const href = label.startsWith("www.") ? `https://${label}` : label;
    let valid = false;

    try {
      valid = new URL(href).hostname !== "";
    } catch {
      valid = false;
    }

    if (!valid) continue;

    parts.push(topic.slice(end, start));
    parts.push(
      <a key={start} href={href} target="_blank" rel="noopener noreferrer">
        {label}
      </a>,
    );
    end = start + label.length;
  }

  parts.push(topic.slice(end));

  return <>{parts}</>;
}
