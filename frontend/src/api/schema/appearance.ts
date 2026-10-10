import { Schema } from "effect";
import type { AppearancePreferences } from "../../gen/AppearancePreferences.ts";
import { TextSize, Theme } from "./me.ts";

const PersonalAppearance = Schema.Struct({
  version: Schema.Literal(1),
  palette: Schema.optionalKey(
    Schema.Literals(["smartfire", "graphite", "ocean", "forest", "ember", "rose"]),
  ),
  font: Schema.optionalKey(Schema.Literals(["inter", "system", "atkinson", "serif", "mono"])),
  density: Schema.optionalKey(Schema.Literals(["comfortable", "compact"])),
  motion: Schema.optionalKey(Schema.Literals(["system", "reduce", "full"])),
  tokens: Schema.optionalKey(Schema.Record(Schema.String, Schema.String)),
});

export type PersonalAppearance = typeof PersonalAppearance.Type;

/** Newer documents stay opaque until this client understands their version. */
export function personalAppearance(value: AppearancePreferences | null): PersonalAppearance | null {
  try {
    return Schema.decodeUnknownSync(PersonalAppearance)(value);
  } catch {
    return null;
  }
}

const AccountBoot = Schema.fromJsonString(
  Schema.Struct({
    theme: Schema.optionalKey(Theme),
    textSize: Schema.optionalKey(TextSize),
    appearancePreferences: Schema.optionalKey(Schema.Json),
  }),
);

export function appearanceBoot(text: string): typeof AccountBoot.Type {
  try {
    return Schema.decodeUnknownSync(AccountBoot)(text);
  } catch {
    return {};
  }
}
