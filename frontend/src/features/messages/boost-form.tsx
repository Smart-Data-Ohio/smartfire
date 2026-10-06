import { type FormEvent, useState } from "react";
import type { MessageDTO } from "../../store/model.ts";
import type { ActionError } from "../../sync/run.ts";
import { Button } from "../../ui/button.tsx";
import { TextField } from "../../ui/text-field.tsx";
import { addBoost } from "./commands.ts";

/** The server's limit on a boost's text (`Boost`: up to 16 characters, trimmed). */
export const BOOST_LIMIT = 16;

interface BoostFormProps {
  readonly message: MessageDTO;
  readonly onDone: () => void;
}

/** A short free-text cheer under the message ("nice work"), up to 16 characters. */
export function BoostForm({ message, onDone }: BoostFormProps) {
  const [text, setText] = useState("");
  const [error, setError] = useState<string | undefined>(undefined);
  const [attempt, setAttempt] = useState(0);
  const [sending, setSending] = useState(false);
  const trimmed = text.trim();

  const submit = (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();

    if (trimmed === "") {
      setError("Write a few words first");
      setAttempt((count) => count + 1);

      return;
    }

    setSending(true);

    addBoost(message, trimmed).then(onDone, (failure: ActionError) => {
      setSending(false);
      setError(failure.message);
      setAttempt((count) => count + 1);
    });
  };

  return (
    <form className="boost-form" onSubmit={submit}>
      <TextField
        label="Boost"
        hint={`${BOOST_LIMIT - text.length} characters left`}
        error={error}
        attempt={attempt}
        value={text}
        maxLength={BOOST_LIMIT}
        placeholder="nice work"
        autoComplete="off"
        data-autofocus
        onChange={(event) => {
          setText(event.target.value);
          setError(undefined);
        }}
      />
      <div className="boost-form-actions">
        <Button variant="ghost" size="sm" onClick={onDone}>
          Cancel
        </Button>
        <Button type="submit" variant="primary" size="sm" icon="rocket" loading={sending}>
          Boost
        </Button>
      </div>
    </form>
  );
}
