/** Private client observations never become part of the wire DTOs or request bodies. */
interface Observation {
  readonly independent: number;
  readonly row: number;
}

const observations = new WeakMap<object, Observation>();

let clock = 0;

export function nextObservation(): number {
  clock += 1;

  return clock;
}

export function observationOf<T extends object>(value: T): number | undefined {
  return observations.get(value)?.independent;
}

export function rowObservationOf<T extends object>(value: T): number | undefined {
  return observations.get(value)?.row;
}

export function copyObservation<From extends object, To extends object>(from: From, to: To): To {
  const observation = observations.get(from);

  if (observation !== undefined) {
    observations.set(to, observation);
  }

  return to;
}

export function observeObject<T extends object>(
  value: T,
  independent: number,
  row = independent,
): T {
  observations.set(value, { independent, row });

  return value;
}

/** The response has already been validated; mark every nested object without guessing DTO types. */
export function observeResponse<T>(value: T, observation: number): void {
  if (!(value instanceof Object)) {
    return;
  }

  observations.set(value, { independent: observation, row: observation });

  for (const child of Object.values(value)) {
    observeResponse(child, observation);
  }
}
