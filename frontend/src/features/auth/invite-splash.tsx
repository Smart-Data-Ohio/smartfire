import { useState } from "react";
import "./invite-splash.css";

export function InviteSplash({ url }: { readonly url: string | null }) {
  const [broken, setBroken] = useState<string | null>(null);

  return (
    <div className="invite-splash" aria-hidden="true">
      {url === null || url === broken ? null : (
        <img src={url} alt="" draggable={false} onError={() => setBroken(url)} />
      )}
    </div>
  );
}
