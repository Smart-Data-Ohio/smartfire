// A signal-ended child has exitCode === null too. Never wait for a second exit.
export function running(child) {
  return child && child.exitCode === null && child.signalCode === null;
}
export async function stopChild(child) {
  if(!running(child)) return;
  const stopped=new Promise(resolve=>child.once('exit',resolve));
  child.kill('SIGTERM');
  await stopped;
}
