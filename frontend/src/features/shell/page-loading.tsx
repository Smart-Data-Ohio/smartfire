import { Skeleton } from "../../ui/skeleton.tsx";

/** The existing home-page loading state, also used while resolving a conversation link. */
export function PageLoading() {
  return (
    <div className="home-empty" aria-busy="true">
      <Skeleton width={180} height={14} />
    </div>
  );
}
