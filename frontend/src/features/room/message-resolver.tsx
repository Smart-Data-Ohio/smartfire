import { useLocation, useParams, useRouter } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { actions } from "../../sync/runtime.ts";
import { PageNotFound } from "../shell/not-found.tsx";
import { PageLoading } from "../shell/page-loading.tsx";
import { messageDestination } from "./message-destination.ts";
import { messageLinkSnapshot } from "./message-link.ts";

type Resolution =
  | { readonly kind: "loading"; readonly messageId: number }
  | { readonly kind: "not-found"; readonly messageId: number };

/** A bare message URL learns its room/thread through the same reachable-message API as forwards. */
export function MessageResolver() {
  const params = useParams({ strict: false });
  const messageId = params.messageId ?? 0;
  const router = useRouter();
  const location = useLocation({ select: () => router.history.location });
  const bare = messageLinkSnapshot(location.href);
  const incoming = location.state.smartfireMessageLink;
  const link = incoming?.messageId === bare?.messageId ? incoming : bare;
  const search = link?.search ?? null;
  const hash = link?.hash ?? "";
  const [resolution, setResolution] = useState<Resolution>({ kind: "loading", messageId });

  useEffect(() => {
    if (search === null) {
      return;
    }

    let current = true;

    setResolution({ kind: "loading", messageId });
    void actions.messages.read(messageId).then(
      ({ message }) => {
        if (current) {
          router.history.replace(messageDestination(message, search, hash));
        }
      },
      () => {
        if (current) {
          setResolution({ kind: "not-found", messageId });
        }
      },
    );

    return () => {
      current = false;
    };
  }, [messageId, search, hash, router]);

  return resolution.messageId === messageId && resolution.kind === "not-found" ? (
    <PageNotFound />
  ) : (
    <PageLoading />
  );
}
