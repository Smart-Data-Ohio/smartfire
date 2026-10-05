// Preserve the pinned Rails within blocks and keyboard-helper ancestry.
export const pickerFrame = p => p.locator('#direct_rooms_control')
export const pickerFilter = p => pickerFrame(p).locator('#dm_picker_filter')
export const pickerRows = p => pickerFrame(p).locator('.dm-picker__row:not([hidden]):visible')
export const pickerEmpty = p => pickerFrame(p).locator("[data-dm-picker-target='empty']")
export const pickerEmptyVisible = p => pickerFrame(p).locator("[data-dm-picker-target='empty']:not([hidden]):visible")
export const tourKey = p => p.locator(".tour__card [data-tour-target='next']")
export const statusField = (p, selector) => p.locator('#user_card').locator(selector)
