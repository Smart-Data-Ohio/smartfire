/**
 * The first-join device check: pick a microphone (with a live meter), a speaker and a camera
 * (with a preview) before anything is published. Only browsers that have never granted the
 * microphone see it; the controller decides.
 */
import { useEffect, useRef } from "react";
import { Button } from "../../ui/button.tsx";
import { Dialog } from "../../ui/dialog.tsx";
import { callController } from "./call-controller.ts";
import { useCall } from "./call-store.ts";
import { DevicePickers, MeterBar } from "./device-pickers.tsx";

function Preview({ stream }: { readonly stream: MediaStream | null }) {
  const video = useRef<HTMLVideoElement>(null);

  useEffect(() => {
    const element = video.current;

    if (element !== null) {
      element.srcObject = stream;
    }
  }, [stream]);

  return (
    <div className="huddle-preview" data-empty={stream === null || undefined}>
      {stream === null ? (
        <span className="huddle-preview-empty">Camera preview is off</span>
      ) : (
        <video ref={video} autoPlay muted playsInline aria-label="Your camera preview" />
      )}
    </div>
  );
}

export default function PrejoinDialog() {
  const prejoin = useCall((state) => state.prejoin);
  const roomName = useCall((state) => state.roomName);
  const devices = useCall((state) => state.devices);
  const meter = useCall((state) => state.meter);

  if (prejoin === null) {
    return null;
  }

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) {
          void callController.leave();
        }
      }}
      title={`Join ${roomName}`}
      description="Check your microphone and camera. Nothing is shared until you join."
      footer={
        <>
          <Button variant="secondary" onClick={() => void callController.leave()}>
            Cancel
          </Button>
          {prejoin.retry ? (
            <Button variant="secondary" onClick={() => void callController.retryPrejoin()}>
              Try again
            </Button>
          ) : null}
          <Button
            variant="primary"
            icon="headphones"
            disabled={!prejoin.canJoin}
            data-autofocus
            onClick={() => void callController.confirmPrejoin()}
          >
            Join
          </Button>
        </>
      }
    >
      <div className="huddle-prejoin">
        <Preview stream={prejoin.preview} />
        <DevicePickers
          devices={devices}
          selected={prejoin.selected}
          onSelect={(kind, deviceId) => void callController.selectDevice(kind, deviceId)}
          cameraOptional
        />
        <MeterBar level={meter} label="Microphone level" />
        {prejoin.error === null ? null : (
          <p className="huddle-prejoin-error" role="alert">
            {prejoin.error}
          </p>
        )}
      </div>
    </Dialog>
  );
}
