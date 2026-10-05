// Preserve the pinned Rails within blocks and keyboard-helper ancestry.
export const pickerFrame = p => p.locator('#direct_rooms_control')
export const pickerFilter = p => pickerFrame(p).locator('#dm_picker_filter')
export const pickerRows = p => pickerFrame(p).locator('.dm-picker__row:not([hidden]):visible')
export const pickerEmpty = p => pickerFrame(p).locator("[data-dm-picker-target='empty']")
export const pickerEmptyVisible = p => pickerFrame(p).locator("[data-dm-picker-target='empty']:not([hidden]):visible")
export const tourKey = p => p.locator(".tour__card [data-tour-target='next']")
export const statusField = (p, selector) => p.locator('#user_card').locator(selector)

// people_group_dms_test.rb:302/304 measure the DOM-first match even if CSS hides
// it. These are document queries outside Rails' picker within block; selector
// visibility and Playwright boundingBox() would change the geometry predicate.
export const pickerGeometry = p => p.evaluate(() => ({
  height: document.querySelector('.dm-picker__row:not([hidden])').getBoundingClientRect().height,
  avatar: document.querySelector('.dm-picker__row:not([hidden]) .avatar').getBoundingClientRect().width
}))
