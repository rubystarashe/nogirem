import { execFile } from "node:child_process"
import { promisify } from "node:util"

const execFileAsync = promisify(execFile)

export async function queryMabinogiVerticalSync() {
  try {
    const { stdout } = await execFileAsync(
      "reg.exe",
      [
        "query",
        "HKCU\\Software\\Nexon\\Mabinogi",
        "/v",
        "VerticalSync",
      ],
      { windowsHide: true, timeout: 10000 },
    )
    const match = stdout.match(/VerticalSync\s+(REG_DWORD|REG_SZ)\s+(0x[0-9a-f]+|\d+)/i)
    if (!match) {
      return {
        available: false,
        reason: "VerticalSync 레지스트리 값을 찾지 못했습니다",
      }
    }
    const rawValue = match[2]
    const value = Number.parseInt(rawValue, rawValue.toLowerCase().startsWith("0x") ? 16 : 10)
    return {
      available: true,
      value,
      enabled: value !== 0,
      registryType: match[1].toUpperCase(),
      location: "HKCU\\Software\\Nexon\\Mabinogi\\VerticalSync",
    }
  } catch (error) {
    return {
      available: false,
      reason: error.message,
    }
  }
}

export async function setMabinogiVerticalSync(enabled) {
  await execFileAsync(
    "reg.exe",
    [
      "add",
      "HKCU\\Software\\Nexon\\Mabinogi",
      "/v",
      "VerticalSync",
      "/t",
      "REG_SZ",
      "/d",
      enabled ? "1" : "0",
      "/f",
    ],
    { windowsHide: true, timeout: 10000 },
  )
  const result = await queryMabinogiVerticalSync()
  if (!result.available || result.enabled !== enabled) {
    throw new Error("마비노기 수직 동기화 설정 검증에 실패했습니다")
  }
  return result
}
