import { useLocation, useNavigate, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";
import { actions } from "../../sync/runtime.ts";
import { PageNotFound } from "../shell/not-found.tsx";
import { PageLoading } from "../shell/page-loading.tsx";
import { messageDestination } from "./message-destination.ts";

type Resolution =
  | { readonly kind: "loading"; readonly messageId: number }
  | { readonly kind: "not-found"; readonly messageId: number };

/** A bare message URL learns its room/thread through the same reachable-message API as forwards. */
export function MessageResolver() {
  const params = useParams({ strict: false });
  const messageId = params.messageId ?? 0;
  const { searchStr, hash } = useLocation();
  const navigate = useNavigate();
  const [resolution, setResolution] = useState<Resolution>({ kind: "loading", messageId });

  useEffect(() => {
    let current = true;

    setResolution({ kind: "loading", messageId });
    void actions.messages.read(messageId).then(
      ({ message }) => {
        if (current) {
          void navigate({ href: messageDestination(message, searchStr, hash), replace: true });
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
  }, [messageId, searchStr, hash, navigate]);

  return resolution.messageId === messageId && resolution.kind === "not-found" ? (
    <PageNotFound />
  ) : (
    <PageLoading />
  );
}
