import { matchRange, normalizeQuery } from "./match.ts";
import "./highlighted.css";

/** The label with the typed text marked, when it appears as one run (accents folded). */
export function Highlighted({ text, query }: { readonly text: string; readonly query: string }) {
  const range = matchRange(text, normalizeQuery(query));

  if (range === null) {
    return text;
  }

  const [start, end] = range;

  return (
    <>
      {text.slice(0, start)}
      <mark className="match-mark">{text.slice(start, end)}</mark>
      {text.slice(end)}
    </>
  );
}
