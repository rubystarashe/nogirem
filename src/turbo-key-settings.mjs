export const defaultTurboKeyCodes = []
export const defaultTurboKeyIntervalMs = 1
export const turboKeyIntervalOptions = [1, 3, 5, 10, 20, 30]

const selectableTurboKeyCodes = new Set([
  0x08, 0x09, 0x0d, 0x1b, 0x20,
  0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x2d, 0x2e,
  ...Array.from({ length: 10 }, (_, index) => 0x30 + index),
  ...Array.from({ length: 26 }, (_, index) => 0x41 + index),
  ...Array.from({ length: 16 }, (_, index) => 0x60 + index),
  ...Array.from({ length: 12 }, (_, index) => 0x70 + index),
  0xba, 0xbb, 0xbc, 0xbd, 0xbe, 0xbf, 0xc0, 0xdb, 0xdc, 0xdd, 0xde,
])

export function normalizeTurboKeyCodes(value) {
  if (!Array.isArray(value)) return [...defaultTurboKeyCodes]
  return [...new Set(
    value.filter(code => Number.isInteger(code) && selectableTurboKeyCodes.has(code)),
  )].sort((left, right) => left - right)
}

export function normalizeTurboKeyIntervalMs(value) {
  const interval = Number(value)
  return turboKeyIntervalOptions.includes(interval)
    ? interval
    : defaultTurboKeyIntervalMs
}
