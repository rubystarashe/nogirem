export const blackboxCodecOptions = ["h264", "hevc"]
export const blackboxCapacityOptions = [20, 50, 100, 200]
export const blackboxClipDurationOptions = [30, 60, 120]
export const blackboxFrameRateOptions = [30, 60]
export const defaultBlackboxSetting = Object.freeze({
  enabled: false,
  codec: "h264",
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
  if (normalized.codec === "hevc") return normalized.fps === 60 ? 8 : 5
  return normalized.fps === 60 ? 12 : 7
}

export function normalizeBlackboxSetting(value) {
  const codec = blackboxCodecOptions.includes(value?.codec)
    ? value.codec
    : defaultBlackboxSetting.codec
  return {
    enabled: Boolean(value?.enabled),
    codec,
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
