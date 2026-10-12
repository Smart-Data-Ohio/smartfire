import type { Attachment } from "../../src/gen/Attachment.ts";
import type { GifCategoryList } from "../../src/gen/GifCategoryList.ts";
import type { GifConfig } from "../../src/gen/GifConfig.ts";
import type { GifList } from "../../src/gen/GifList.ts";
import type { GifSuggestions } from "../../src/gen/GifSuggestions.ts";
import { GIF_BYTES, GIF_FIXTURES, GIF_STILL_BYTES } from "../fixtures/gifs.ts";
import type { MockBinaryRequest, MockBinaryResponse, MockResponse } from "../http.ts";
import { notFound, validation } from "../http.ts";
import { route } from "./context.ts";

export function createGifs(enabled: boolean) {
  const requireEnabled = () => {
    if (!enabled) throw notFound("GIFs are disabled on this server");
  };
  const queryOf = (query: URLSearchParams) => {
    const text = (query.get("q") ?? "").trim();

    if (text === "" || text.length > 200) throw validation("q", "must contain 1 to 200 characters");

    return text.toLowerCase();
  };
  const list = (query: URLSearchParams, search: boolean): MockResponse => {
    requireEnabled();
    const text = search ? queryOf(query) : "";
    const page = Number(query.get("page") ?? "1");

    if (!Number.isInteger(page) || page < 1 || page > 10000)
      throw validation("page", "must be between 1 and 10000");

    const json: GifList = {
      items:
        page === 1 ? GIF_FIXTURES.filter((item) => item.title.toLowerCase().includes(text)) : [],
      page,
      hasNext: false,
    };

    return { status: 200, json };
  };
  const suggestions = (query: URLSearchParams): MockResponse => {
    requireEnabled();
    const text = queryOf(query);
    const json: GifSuggestions = {
      suggestions: ["hello", "hello there", "celebration", "cheer"].filter((term) =>
        term.includes(text),
      ),
    };

    return { status: 200, json };
  };

  return {
    routes: [
      route("GET", /^\/gifs\/config$/, () => {
        const json: GifConfig = { enabled };

        return { status: 200, json };
      }),
      route("GET", /^\/gifs\/search$/, ({ query }) => list(query, true)),
      route("GET", /^\/gifs\/trending$/, ({ query }) => list(query, false)),
      route("GET", /^\/gifs\/(suggestions|autocomplete)$/, ({ query }) => suggestions(query)),
      route("GET", /^\/gifs\/categories$/, () => {
        requireEnabled();
        const json: GifCategoryList = {
          categories: GIF_FIXTURES.map((item) => ({
            name: item.title,
            query: item.title,
            previewUrl: item.stillUrl,
          })),
        };

        return { status: 200, json };
      }),
    ],
    attachment: (slug: string): Attachment => {
      requireEnabled();
      const item = GIF_FIXTURES.find((gif) => gif.slug === slug);

      if (item === undefined) throw validation("gifSlug", "must exist");

      const url = `/mock/files/gifs/${item.slug}.gif`;

      return {
        spoiler: false,
        filename: `${item.title.replaceAll(" ", "-")}.gif`,
        contentType: "image/gif",
        byteSize: GIF_BYTES.length,
        width: item.width,
        height: item.height,
        preview: "image",
        url,
        downloadUrl: `${url}?disposition=attachment`,
        thumbnailUrl: null,
      };
    },
    handleBinary: (request: MockBinaryRequest): MockBinaryResponse | null => {
      const url = new URL(request.path, "https://campfire.test");
      const stored = url.pathname.startsWith("/mock/files/gifs/");

      if (!stored && url.pathname !== "/api/v1/gifs/media") return null;

      const media = stored ? url.pathname : url.searchParams.get("u");
      const item = GIF_FIXTURES.find(
        (gif) => media?.endsWith(`/${gif.slug}.gif`) || media?.endsWith(`/${gif.slug}.png`),
      );
      const empty = { contentType: null, bytes: new Uint8Array(), headers: {} };

      if (request.method !== "GET" || (!stored && !enabled) || item === undefined)
        return { ...empty, status: 404 };

      if (
        !stored &&
        media !== `https://static.klipy.com/mock/${item.slug}.gif` &&
        media !== `https://static.klipy.com/mock/${item.slug}.png`
      )
        return { ...empty, status: 422 };

      const still = media?.endsWith(".png") ?? false;

      return {
        status: 200,
        contentType: still ? "image/png" : "image/gif",
        bytes: still ? GIF_STILL_BYTES : GIF_BYTES,
        headers: { "X-Content-Type-Options": "nosniff", "Cache-Control": "no-store" },
      };
    },
  };
}
