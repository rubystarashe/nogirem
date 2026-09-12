const minimumNvidiaDriverForDxvk3 = 575.51

export function parseWindowsNvidiaDriverVersion(driverVersion) {
  const parts = String(driverVersion ?? "").split(".").map(Number)
  if (parts.length !== 4 || parts.some(part => !Number.isInteger(part) || part < 0)) {
    return null
  }
  const versionDigits = `${parts[2] % 10}${String(parts[3]).padStart(4, "0")}`
  const version = Number(versionDigits) / 100
  return Number.isFinite(version) ? version : null
}

export function assessDxvkCompatibility(version, gpuInfo) {
  const major = Number.parseInt(/^v?(\d+)/i.exec(String(version ?? ""))?.[1] ?? "", 10)
  if (!Number.isInteger(major) || major < 3) {
    return { compatible: true, checked: true, reason: null }
  }

  const nvidiaGpu = [...(gpuInfo?.gpuDevice ?? [])]
    .filter(gpu => Number(gpu?.vendorId) === 0x10de)
    .sort((left, right) => Number(right?.gpuPreference ?? 0) - Number(left?.gpuPreference ?? 0))[0]
  if (!nvidiaGpu) {
    return { compatible: true, checked: false, reason: null }
  }

  const driverVersion = parseWindowsNvidiaDriverVersion(nvidiaGpu.driverVersion)
  if (driverVersion == null) {
    return { compatible: true, checked: false, reason: null }
  }
  if (driverVersion < minimumNvidiaDriverForDxvk3) {
    return {
      compatible: false,
      checked: true,
      driverVersion: driverVersion.toFixed(2),
      minimumDriverVersion: minimumNvidiaDriverForDxvk3.toFixed(2),
      reason: `NVIDIA ${driverVersion.toFixed(2)} 드라이버는 DXVK 3.x 요구사항을 충족하지 않습니다`,
    }
  }
  return {
    compatible: true,
    checked: true,
    driverVersion: driverVersion.toFixed(2),
    minimumDriverVersion: minimumNvidiaDriverForDxvk3.toFixed(2),
    reason: null,
  }
}
