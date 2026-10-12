import { Effect } from "effect";
import type { CreateSticker } from "../gen/CreateSticker.ts";
import type { StickerCatalog } from "../gen/StickerCatalog.ts";
import type { UpdateSticker } from "../gen/UpdateSticker.ts";
import { call, get } from "./call.ts";
import { StickerCatalog as StickerCatalogSchema } from "./schema/sticker.ts";
import { wire } from "./wire.ts";

const catalogReply = wire<StickerCatalog>(StickerCatalogSchema);

export const stickerCatalog = Effect.fn("api.stickerCatalog")(function* () {
  return yield* call(get("/stickers"), catalogReply);
});

export const createSticker = Effect.fn("api.createSticker")(function* (body: CreateSticker) {
  return yield* call({ method: "POST", path: "/admin/stickers", body }, catalogReply);
});

export const updateSticker = Effect.fn("api.updateSticker")(function* (
  id: number,
  body: UpdateSticker,
) {
  return yield* call({ method: "PATCH", path: `/admin/stickers/${id}`, body }, catalogReply);
});

export const destroySticker = Effect.fn("api.destroySticker")(function* (id: number) {
  return yield* call({ method: "DELETE", path: `/admin/stickers/${id}` }, catalogReply);
});
