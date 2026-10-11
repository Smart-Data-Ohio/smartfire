import type { WebhookEmbed as Embed } from "../../gen/WebhookEmbed.ts";
import { formatFull } from "../../lib/time.ts";

/** `#rrggbb` for an embed's 24-bit colour, or nothing for none or one out of range. */
export function embedColor(color: number | undefined): string | undefined {
  if (color === undefined || !Number.isInteger(color) || color < 0 || color > 0xffffff) {
    return undefined;
  }

  return `#${color.toString(16).padStart(6, "0")}`;
}

/** Only an absolute web link becomes one; the server checks the same, this is belt and braces. */
function webLink(url: string | undefined): string | null {
  return url !== undefined && /^https?:\/\//i.test(url) ? url : null;
}

/**
 * One of a webhook message's embeds, Discord's rich-embed shape cut down to text: a title (linked
 * when it has a URL), the description, name/value fields (inline ones share a row), and a footer
 * with its time. Every value is plain text; a colour, when given, paints the left edge.
 */
export function WebhookEmbed({ embed }: { readonly embed: Embed }) {
  const color = embedColor(embed.color);
  const link = webLink(embed.url);
  const style = color === undefined ? undefined : { "--embed-color": color };

  const stamp =
    embed.timestamp === undefined || Number.isNaN(Date.parse(embed.timestamp))
      ? null
      : formatFull(embed.timestamp);

  return (
    <section className="card webhook-embed" style={style} aria-label={embed.title ?? "Embed"}>
      {embed.title === undefined ? null : link === null ? (
        <span className="webhook-embed-title">{embed.title}</span>
      ) : (
        <a className="webhook-embed-title" href={link} target="_blank" rel="noopener noreferrer">
          {embed.title}
        </a>
      )}
      {embed.description === undefined ? null : (
        <p className="webhook-embed-description">{embed.description}</p>
      )}
      {embed.fields === undefined || embed.fields.length === 0 ? null : (
        <dl className="webhook-embed-fields">
          {embed.fields.map((field, index) => (
            <div
              // biome-ignore lint/suspicious/noArrayIndexKey: fields are an ordered list with no id
              key={index}
              className="webhook-embed-field"
              data-inline={field.inline === true || undefined}
            >
              <dt>{field.name}</dt>
              <dd>{field.value}</dd>
            </div>
          ))}
        </dl>
      )}
      {embed.footer === undefined && stamp === null ? null : (
        <p className="card-subtle webhook-embed-footer">
          {embed.footer?.text}
          {embed.footer !== undefined && stamp !== null ? " · " : null}
          {stamp}
        </p>
      )}
    </section>
  );
}
