import { highlight } from "./tokenize.ts";

/**
 * The code-highlight worker: tokenizing runs here so that loading grammars and colouring a long
 * block never holds up scrolling or typing (classic's web/script/code-highlighter/worker.mjs).
 */
export interface HighlightRequest {
  readonly id: number;
  readonly source: string;
  readonly label: string | undefined;
}

globalThis.addEventListener("message", (event: MessageEvent<HighlightRequest>) => {
  const { id, source, label } = event.data;

  highlight(source, label).then(
    (result) => globalThis.postMessage({ id, result }),
    () => globalThis.postMessage({ id, result: null }),
  );
});
