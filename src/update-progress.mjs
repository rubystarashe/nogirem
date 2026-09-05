const DOWNLOAD_PROGRESS_LIMIT = 99.9

function clampProgress(value, maximum = 100) {
  return Math.max(0, Math.min(maximum, Number(value) || 0))
}

export function advanceDownloadProgress(previousPercent, reportedPercent) {
  const previous = clampProgress(previousPercent, DOWNLOAD_PROGRESS_LIMIT)
  const reported = clampProgress(reportedPercent, DOWNLOAD_PROGRESS_LIMIT)
  return Math.max(previous, reported)
}
