import { createMessage } from "../api/endpoints.ts";
import * as gifs from "../api/gif-endpoints.ts";
import { createThreadMessage } from "../api/thread-endpoints.ts";
import type { CreateMessage } from "../gen/CreateMessage.ts";
import type { GifCategoryList } from "../gen/GifCategoryList.ts";
import type { GifConfig } from "../gen/GifConfig.ts";
import type { GifList } from "../gen/GifList.ts";
import type { GifSuggestions } from "../gen/GifSuggestions.ts";
import type { MessageDTO } from "../gen/MessageDTO.ts";
import { mutations } from "../store/store.ts";
import { runAction } from "./runtime.ts";

export interface SendGif {
  readonly roomId: number;
  readonly threadId?: number | undefined;
  readonly slug: string;
  readonly clientMessageId: string;
  readonly markdownSource?: string | undefined;
}

/** Picker requests stay local to its caller. Sending uses the ordinary message endpoint. */
export const gifActions = {
  config: (): Promise<GifConfig> => runAction(gifs.gifConfig()),
  search: (query: string, page = 1): Promise<GifList> => runAction(gifs.searchGifs(query, page)),
  trending: (page = 1): Promise<GifList> => runAction(gifs.trendingGifs(page)),
  categories: (): Promise<GifCategoryList> => runAction(gifs.gifCategories()),
  suggestions: (query: string): Promise<GifSuggestions> => runAction(gifs.gifSuggestions(query)),
  autocomplete: (query: string): Promise<GifSuggestions> => runAction(gifs.gifAutocomplete(query)),
  send: async (input: SendGif): Promise<MessageDTO> => {
    const body: CreateMessage = {
      clientMessageId: input.clientMessageId,
      markdownSource: input.markdownSource ?? "",
      replyToMessageId: null,
      replyNotifyAuthor: null,
      attachmentSignedId: null,
      gifSlug: input.slug,
    };

    const message = await runAction(
      input.threadId === undefined
        ? createMessage(input.roomId, body)
        : createThreadMessage(input.threadId, body),
    );

    mutations.receiveMessage(message);

    return message;
  },
};
