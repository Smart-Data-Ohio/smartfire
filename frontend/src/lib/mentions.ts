/** The server tags each user mention chip with its user's id. */
export function mentionsViewer(bodyHtml: string, viewerId: number | null): boolean {
  return viewerId !== null && bodyHtml.includes(`data-user-id="${viewerId}"`);
}
