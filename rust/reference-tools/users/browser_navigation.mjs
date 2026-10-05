// Initial navigation is separate from the original controller/selector assertions.
export function visit(page, url) {
  // A pending avatar/preview must not gate the controller/selector assertions.
  // Those readiness assertions below the visit keep their original deadlines.
  return page.goto(url, {waitUntil:'domcontentloaded'})
}
