import { Schema } from "effect";
import type { CreatedFizzyCard as GeneratedCreatedFizzyCard } from "../../gen/CreatedFizzyCard.ts";
import type { CreateFizzyCard as GeneratedCreateFizzyCard } from "../../gen/CreateFizzyCard.ts";
import type { FizzyBoard as GeneratedFizzyBoard } from "../../gen/FizzyBoard.ts";
import type { FizzyMessageCardForm as GeneratedFizzyMessageCardForm } from "../../gen/FizzyMessageCardForm.ts";
import { MessageDTO } from "./message.ts";
import type { Assert, Pinned } from "./pin.ts";

export const FizzyBoard = Schema.Struct({ id: Schema.String, name: Schema.String });

export type FizzyBoardPin = Assert<Pinned<typeof FizzyBoard, GeneratedFizzyBoard>>;

export const FizzyMessageCardForm = Schema.Struct({
  connected: Schema.Boolean,
  boards: Schema.Array(FizzyBoard),
  title: Schema.String,
  description: Schema.String,
  excerpt: Schema.String,
  authorName: Schema.String,
  roomDisplayName: Schema.String,
  fizzyUserName: Schema.String,
  accountName: Schema.String,
});

export type FizzyMessageCardFormPin = Assert<
  Pinned<typeof FizzyMessageCardForm, GeneratedFizzyMessageCardForm>
>;

export const CreateFizzyCard = Schema.Struct({
  boardId: Schema.String,
  title: Schema.String,
  description: Schema.String,
});

export type CreateFizzyCardPin = Assert<Pinned<typeof CreateFizzyCard, GeneratedCreateFizzyCard>>;

export const CreatedFizzyCard = Schema.Struct({
  number: Schema.String,
  url: Schema.String,
  message: MessageDTO,
  notice: Schema.String,
});

export type CreatedFizzyCardPin = Assert<
  Pinned<typeof CreatedFizzyCard, GeneratedCreatedFizzyCard>
>;
