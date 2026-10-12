import { Schema } from "effect";
import type { CreateSticker as GeneratedCreateSticker } from "../../gen/CreateSticker.ts";
import type { MessageSticker as GeneratedMessageSticker } from "../../gen/MessageSticker.ts";
import type { Sticker as GeneratedSticker } from "../../gen/Sticker.ts";
import type { StickerCatalog as GeneratedStickerCatalog } from "../../gen/StickerCatalog.ts";
import type { UpdateSticker as GeneratedUpdateSticker } from "../../gen/UpdateSticker.ts";
import { UserId } from "./ids.ts";
import type { Assert, Pinned } from "./pin.ts";
import { Timestamp } from "./time.ts";

export const StickerId = Schema.Int.pipe(Schema.brand("StickerId"));

export type StickerId = typeof StickerId.Type;

export const Sticker = Schema.Struct({
  id: StickerId,
  name: Schema.String,
  description: Schema.String,
  tags: Schema.String,
  imageUrl: Schema.String,
  stillUrl: Schema.String,
  animated: Schema.Boolean,
  uploaderId: UserId,
  createdAt: Timestamp,
  updatedAt: Timestamp,
});

export type Sticker = typeof Sticker.Type;

export type StickerPin = Assert<Pinned<typeof Sticker, GeneratedSticker>>;

export const StickerCatalog = Schema.Struct({
  stickers: Schema.Array(Sticker),
  capacity: Schema.Int,
});

export type StickerCatalog = typeof StickerCatalog.Type;

export type StickerCatalogPin = Assert<Pinned<typeof StickerCatalog, GeneratedStickerCatalog>>;

export const CreateSticker = Schema.Struct({
  name: Schema.String,
  description: Schema.String,
  tags: Schema.String,
  signedId: Schema.NullOr(Schema.String),
});

export type CreateStickerPin = Assert<Pinned<typeof CreateSticker, GeneratedCreateSticker>>;

export const UpdateSticker = Schema.Struct({
  name: Schema.String,
  description: Schema.String,
  tags: Schema.String,
});

export type UpdateStickerPin = Assert<Pinned<typeof UpdateSticker, GeneratedUpdateSticker>>;

export const MessageSticker = Schema.Union([
  Schema.Struct({
    kind: Schema.Literal("available"),
    id: StickerId,
    name: Schema.String,
    imageUrl: Schema.String,
    stillUrl: Schema.String,
    animated: Schema.Boolean,
  }),
  Schema.Struct({ kind: Schema.Literal("deleted"), name: Schema.String }),
]);

export type MessageSticker = typeof MessageSticker.Type;

export type MessageStickerPin = Assert<Pinned<typeof MessageSticker, GeneratedMessageSticker>>;
