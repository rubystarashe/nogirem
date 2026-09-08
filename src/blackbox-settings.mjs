export const blackboxCodecOptions = ["h264", "hevc"]
export const blackboxQualityOptions = ["auto", "1080p", "1440p", "original"]
export const blackboxCapacityOptions = [20, 50, 100, 200]
export const blackboxClipDurationOptions = [30, 60, 120]
export const blackboxFrameRateOptions = [30, 60]
export const blackboxFeatureAvailable = true
export const defaultBlackboxSetting = Object.freeze({
  featureEnabled: false,
  enabled: false,
  codec: "h264",
  quality: "auto",
  capacityGb: 50,
  clipSeconds: 30,
  fps: 60,
  chunkSeconds: 4,
})

function normalizeOption(value, options, fallback) {
  const normalized = Number(value)
  return options.includes(normalized) ? normalized : fallback
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
  }
}
