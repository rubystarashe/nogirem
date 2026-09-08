export const blackboxCodecOptions = ["h264", "hevc"]
export const blackboxQualityOptions = ["auto", "1080p", "1440p", "original"]
export const blackboxCapacityOptions = [20, 50, 100, 200]
export const blackboxClipDurationOptions = [30, 60, 120]
export const blackboxFrameRateOptions = [30, 60]
export const blackboxFeatureAvailable = true
export const defaultBlackboxShortcut = "CommandOrControl+Shift+F10"
export const defaultBlackboxSetting = Object.freeze({
  featureEnabled: false,
  enabled: false,
  codec: "h264",
  quality: "auto",
  capacityGb: 50,
  clipSeconds: 30,
  fps: 60,
  chunkSeconds: 10,
  shortcut: defaultBlackboxShortcut,
})

function normalizeOption(value, options, fallback) {
  const normalized = Number(value)
  return options.includes(normalized) ? normalized : fallback
}

export function normalizeBlackboxShortcut(value) {
  const tokens = String(value ?? "")
    .split("+")
    .map(token => token.trim())
    .filter(Boolean)
  const modifierAliases = new Map([
    ["commandorcontrol", "CommandOrControl"],
    ["cmdorctrl", "CommandOrControl"],
    ["control", "Control"],
    ["ctrl", "Control"],
    ["alt", "Alt"],
    ["shift", "Shift"],
    ["super", "Super"],
    ["meta", "Super"],
  ])
  const modifiers = []
  let key = ""
  for (const token of tokens) {
    const modifier = modifierAliases.get(token.toLowerCase())
    if (modifier) {
      if (!modifiers.includes(modifier)) modifiers.push(modifier)
      continue
    }
    if (key) return defaultBlackboxShortcut
    const upper = token.toUpperCase()
    if (/^F(?:[1-9]|1\d|2[0-4])$/.test(upper) || /^[A-Z0-9]$/.test(upper)) {
      key = upper
      continue
    }
    const namedKeys = new Map([
      ["space", "Space"],
      ["up", "Up"],
      ["down", "Down"],
      ["left", "Left"],
      ["right", "Right"],
      ["insert", "Insert"],
      ["delete", "Delete"],
      ["home", "Home"],
      ["end", "End"],
      ["pageup", "PageUp"],
      ["pagedown", "PageDown"],
    ])
    key = namedKeys.get(token.toLowerCase()) ?? ""
    if (!key) return defaultBlackboxShortcut
  }
  if (!modifiers.length || !key) return defaultBlackboxShortcut
  const order = ["CommandOrControl", "Control", "Alt", "Shift", "Super"]
  modifiers.sort((left, right) => order.indexOf(left) - order.indexOf(right))
  return [...modifiers, key].join("+")
}

export function bitrateForBlackboxSetting(setting) {
  const normalized = normalizeBlackboxSetting(setting)
  const quality = normalized.quality === "auto" ? "1440p" : normalized.quality
  const rates = {
    "1080p": {
      h264: normalized.fps === 60 ? 12 : 7,
      hevc: normalized.fps === 60 ? 8 : 5,
    },
    "1440p": {
      h264: normalized.fps === 60 ? 24 : 14,
      hevc: normalized.fps === 60 ? 16 : 10,
    },
    original: {
      h264: normalized.fps === 60 ? 32 : 20,
      hevc: normalized.fps === 60 ? 22 : 14,
    },
  }
  return rates[quality][normalized.codec]
}

export function resolveAutoBlackboxQuality({
  logicalCpuCount = 0,
  totalMemoryBytes = 0,
} = {}) {
  return logicalCpuCount >= 12 && totalMemoryBytes >= 16 * 1024 ** 3
    ? "1440p"
    : "1080p"
}

export function resolveBlackboxQuality(setting, environment) {
  const normalized = normalizeBlackboxSetting(setting)
  return normalized.quality === "auto"
    ? resolveAutoBlackboxQuality(environment)
    : normalized.quality
}

export function maxHeightForBlackboxQuality(quality) {
  if (quality === "original") return 0
  return quality === "1440p" ? 1440 : 1080
}

export function normalizeBlackboxSetting(value) {
  const codec = blackboxCodecOptions.includes(value?.codec)
    ? value.codec
    : defaultBlackboxSetting.codec
  const enabled = Boolean(value?.enabled)
  return {
    featureEnabled: Boolean(value?.featureEnabled || enabled),
    enabled,
    codec,
    quality: blackboxQualityOptions.includes(value?.quality)
      ? value.quality
      : defaultBlackboxSetting.quality,
    capacityGb: normalizeOption(
      value?.capacityGb,
      blackboxCapacityOptions,
      defaultBlackboxSetting.capacityGb,
    ),
    clipSeconds: normalizeOption(
      value?.clipSeconds,
      blackboxClipDurationOptions,
      defaultBlackboxSetting.clipSeconds,
    ),
    fps: normalizeOption(
      value?.fps,
      blackboxFrameRateOptions,
      defaultBlackboxSetting.fps,
    ),
    chunkSeconds: defaultBlackboxSetting.chunkSeconds,
    shortcut: normalizeBlackboxShortcut(value?.shortcut),
  }
}
