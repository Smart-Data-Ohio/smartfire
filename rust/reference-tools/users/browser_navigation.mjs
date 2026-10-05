// Initial navigation is separate from the original controller/selector assertions.
export function visit(page, url) {
  // A pending avatar/preview must not gate the controller/selector assertions.
  // Those readiness assertions below the visit keep their original deadlines.
  return page.goto(url, {waitUntil:'domcontentloaded'})
}

export function waitForController(page, name) {
  return page.waitForFunction(name => {
    const element = document.querySelector(`[data-controller~='${name}']`)
    return element && window.Stimulus?.getControllerForElementAndIdentifier(element, name)
  }, name)
}
