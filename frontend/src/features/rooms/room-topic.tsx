import type { ReactNode } from "react";

function trimUrlEnd(url: string): string {
  const pairs = new Map([
    [")", "("],
    ["]", "["],
    ["}", "{"],
  ]);

  const stack: string[] = [];
  const unmatched = new Set<number>();

  for (const match of url.matchAll(/[()[\]{}]/g)) {
    const bracket = match[0];

    if ("([{".includes(bracket)) {
      stack.push(bracket);
    } else if (stack.at(-1) === pairs.get(bracket)) {
      stack.pop();
    } else {
      unmatched.add(match.index);
    }
  }

  let end = url.length;

  while (end > 0 && (/[.,!?;:]/.test(url.charAt(end - 1)) || unmatched.has(end - 1))) {
    end -= 1;
  }

  return url.slice(0, end);
}

/** Topic text stays literal; only web URLs become links. */
export function RoomTopic({ topic }: { readonly topic: string }) {
  const parts: ReactNode[] = [];
  let end = 0;

  for (const match of topic.matchAll(/(?:https?:\/\/|www\.)[^\s<>"']+/g)) {
    const start = match.index;
    const label = trimUrlEnd(match[0]);

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
