import type { MessageSound } from "../../gen/MessageSound.ts";
import { actions } from "../../sync/runtime.ts";
import { Button } from "../../ui/button.tsx";

export function SoundMessage({ sound }: { readonly sound: MessageSound }) {
  const presentation = sound.presentation;

  return (
    <div className="message-body message-sound">
      <Button
        variant="ghost"
        size="sm"
        aria-label={`Play ${sound.name}`}
        onClick={() => actions.chatSounds.play(sound.url)}
      >
        <span aria-hidden="true">🔊</span>
      </Button>
      {presentation.kind === "text" ? (
        presentation.text
      ) : (
        <img
          src={presentation.url}
          width={presentation.width}
          height={presentation.height}
          alt={sound.name}
        />
      )}
    </div>
  );
}
