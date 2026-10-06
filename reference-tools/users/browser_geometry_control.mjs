import assert from 'node:assert/strict'

// Mutate the real rendered picker, leaving a later matching row measurable.
// A visibility-filtered first() incorrectly skips this DOM-first zero-size row.
export async function hideFirstPickerRow(page) {
  const geometry = await page.evaluate(() => {
    const rows = document.querySelectorAll('.dm-picker__row:not([hidden])')
    rows[0].style.display = 'none'
    return {
      matched: rows.length,
      height: rows[0].getBoundingClientRect().height,
      avatar: rows[0].querySelector('.avatar').getBoundingClientRect().width,
      nextHeight: rows[1].getBoundingClientRect().height,
      nextAvatar: rows[1].querySelector('.avatar').getBoundingClientRect().width
    }
  })
  assert.ok(geometry.matched >= 2, 'INVALID_CONTROL needs a later matching row')
  assert.equal(geometry.height, 0, 'INVALID_CONTROL first row must measure zero')
  assert.equal(geometry.avatar, 0, 'INVALID_CONTROL first avatar must measure zero')
  assert.ok(geometry.nextHeight >= 44, 'INVALID_CONTROL later row must remain valid')
  assert.ok(Math.abs(geometry.nextAvatar - 32) <= 1, 'INVALID_CONTROL later avatar must remain valid')
  console.log(`GEOMETRY_MUTATION phone-first-row ${JSON.stringify(geometry)}`)
}
