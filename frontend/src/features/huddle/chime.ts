export const B5_HZ = 987.77;

const E6_HZ = 1318.51;

function strike(
  audio: BaseAudioContext,
  frequency: number,
  at: number,
  peak: number,
  decay: number,
): void {
  const oscillator = audio.createOscillator();
  const envelope = audio.createGain();

  oscillator.type = "sine";
  oscillator.frequency.value = frequency;
  envelope.gain.setValueAtTime(0.0001, at);
  envelope.gain.exponentialRampToValueAtTime(peak, at + 0.006);
  envelope.gain.exponentialRampToValueAtTime(0.0001, at + decay);
  oscillator.connect(envelope);
  envelope.connect(audio.destination);
  oscillator.start(at);
  oscillator.stop(at + decay + 0.02);
}

export function chime(audio: BaseAudioContext, at: number): void {
  strike(audio, B5_HZ, at, 0.09, 0.5);
  strike(audio, B5_HZ * 2, at, 0.012, 0.18);
  strike(audio, E6_HZ, at + 0.09, 0.08, 0.6);
  strike(audio, E6_HZ * 2, at + 0.09, 0.01, 0.2);
}
