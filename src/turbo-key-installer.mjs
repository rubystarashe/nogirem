import { createHash, randomUUID } from "node:crypto"
import { readFile, mkdir, rename, unlink, writeFile } from "node:fs/promises"
import { dirname, join } from "node:path"

export const turboKeyHelperVersion = "0.1.5"
export const turboKeyHelperProtocolVersion = 1
export const turboKeyHelperAssetName = `turbo-key-helper-win32-x64-v${turboKeyHelperVersion}.exe`

const maximumHelperSize = 2 * 1024 * 1024

function helperPaths(directory) {
  return {
    executablePath: join(directory, "bin", "turbo-key-helper.exe"),
    manifestPath: join(directory, "current.json"),
  }
}

async function readJson(path) {
  try {
    return JSON.parse(await readFile(path, "utf8"))
  } catch (error) {
    if (error?.code === "ENOENT") return null
    throw error
  }
}

async function writeJsonAtomic(path, value) {
  await mkdir(dirname(path), { recursive: true })
  const temporaryPath = `${path}.${process.pid}.${randomUUID()}.tmp`
  await writeFile(temporaryPath, JSON.stringify(value), "utf8")
  await rename(temporaryPath, path)
}

function sha256Buffer(buffer) {
  return createHash("sha256").update(buffer).digest("hex")
}

export function validateTurboKeyExecutable(buffer) {
  if (!Buffer.isBuffer(buffer) || buffer.length < 256) {
    throw new Error("터보 키 실행 파일이 올바르지 않습니다")
  }
  if (buffer.readUInt16LE(0) !== 0x5a4d) {
    throw new Error("터보 키 실행 파일의 MZ 헤더가 올바르지 않습니다")
  }
  const peOffset = buffer.readUInt32LE(0x3c)
  if (peOffset < 0x40 || peOffset + 6 > buffer.length) {
    throw new Error("터보 키 실행 파일의 PE 위치가 올바르지 않습니다")
  }
  if (buffer.readUInt32LE(peOffset) !== 0x00004550) {
    throw new Error("터보 키 실행 파일의 PE 헤더가 올바르지 않습니다")
  }
  if (buffer.readUInt16LE(peOffset + 4) !== 0x8664) {
    throw new Error("터보 키 실행 파일이 Windows x64용이 아닙니다")
  }
  return true
}

export async function getLocalTurboKeyHelper(localSourcePath) {
  try {
    const buffer = await readFile(localSourcePath)
    if (buffer.length > maximumHelperSize) {
      throw new Error("터보 키 실행 파일의 크기가 올바르지 않습니다")
    }
    validateTurboKeyExecutable(buffer)
    return {
      installed: true,
      updateRequired: false,
      executablePath: localSourcePath,
      helperVersion: turboKeyHelperVersion,
      installedVersion: turboKeyHelperVersion,
      protocolVersion: turboKeyHelperProtocolVersion,
      sha256: sha256Buffer(buffer),
      source: "local-development-build",
      reason: null,
    }
  } catch (error) {
    return {
      installed: false,
      updateRequired: false,
      executablePath: localSourcePath,
      helperVersion: turboKeyHelperVersion,
      installedVersion: null,
      reason: error?.code === "ENOENT"
        ? "로컬 터보 키 helper를 먼저 빌드하세요"
        : (error?.message ?? String(error)),
    }
  }
}

export async function getTurboKeyHelperInstallation(directory) {
  const paths = helperPaths(directory)
  const manifest = await readJson(paths.manifestPath)
  if (
    !manifest
    || !/^[a-f0-9]{64}$/i.test(manifest.sha256 ?? "")
  ) {
    return {
      installed: false,
      updateRequired: false,
      executablePath: paths.executablePath,
      helperVersion: turboKeyHelperVersion,
      installedVersion: manifest?.helperVersion ?? null,
      reason: null,
    }
  }

  try {
    const buffer = await readFile(paths.executablePath)
    validateTurboKeyExecutable(buffer)
    const actualSha256 = sha256Buffer(buffer)
    if (actualSha256 !== manifest.sha256.toLowerCase()) {
      throw new Error("터보 키 실행 파일의 무결성 검증에 실패했습니다")
    }
    const updateRequired = manifest.helperVersion !== turboKeyHelperVersion
      || manifest.protocolVersion !== turboKeyHelperProtocolVersion
    return {
      installed: !updateRequired,
      updateRequired,
      executablePath: paths.executablePath,
      helperVersion: turboKeyHelperVersion,
      installedVersion: manifest.helperVersion,
      protocolVersion: manifest.protocolVersion,
      sha256: actualSha256,
      installedAt: manifest.installedAt,
      acceptedAt: manifest.acceptedAt,
      source: manifest.source,
      reason: updateRequired ? "터보 키 업데이트가 필요합니다" : null,
    }
  } catch (error) {
    return {
      installed: false,
      updateRequired: false,
      executablePath: paths.executablePath,
      helperVersion: turboKeyHelperVersion,
      installedVersion: manifest.helperVersion ?? null,
      reason: error?.code === "ENOENT"
        ? null
        : (error?.message ?? String(error)),
    }
  }
}

export async function removeTurboKeyHelper(directory) {
  const paths = helperPaths(directory)
  await Promise.all([
    unlink(paths.executablePath).catch(error => {
      if (error?.code !== "ENOENT") throw error
    }),
    unlink(paths.manifestPath).catch(error => {
      if (error?.code !== "ENOENT") throw error
    }),
  ])
  return getTurboKeyHelperInstallation(directory)
}

async function fetchWithTimeout(fetchImpl, url, options = {}) {
  const controller = new AbortController()
  const timer = setTimeout(() => controller.abort(), 15000)
  try {
    return await fetchImpl(url, {
      ...options,
      signal: controller.signal,
      headers: {
        Accept: "application/vnd.github+json",
        "User-Agent": "nogirem",
        ...options.headers,
      },
    })
  } finally {
    clearTimeout(timer)
  }
}

async function downloadReleaseAsset({
  appVersion,
  owner,
  repo,
  fetchImpl,
}) {
  const releaseUrl = `https://api.github.com/repos/${owner}/${repo}/releases/tags/v${appVersion}`
  const releaseResponse = await fetchWithTimeout(fetchImpl, releaseUrl)
  if (!releaseResponse.ok) {
    throw new Error(`터보 키 다운로드 정보를 가져오지 못했습니다 (${releaseResponse.status})`)
  }
  const release = await releaseResponse.json()
  if (release?.draft || release?.prerelease) {
    throw new Error("정식 Release의 터보 키 실행 파일만 다운로드할 수 있습니다")
  }
  const asset = release?.assets?.find(candidate => (
    candidate?.name === turboKeyHelperAssetName
    && candidate?.state === "uploaded"
  ))
  if (!asset) throw new Error("현재 버전에 맞는 터보 키 다운로드 파일이 없습니다")
  if (!(asset.size > 0 && asset.size <= maximumHelperSize)) {
    throw new Error("터보 키 다운로드 파일의 크기가 올바르지 않습니다")
  }
  const digestMatch = String(asset.digest ?? "").match(/^sha256:([a-f0-9]{64})$/i)
  if (!digestMatch) throw new Error("터보 키 다운로드 파일의 SHA-256 정보가 없습니다")
  const assetUrl = new URL(asset.browser_download_url)
  if (assetUrl.protocol !== "https:" || assetUrl.hostname !== "github.com") {
    throw new Error("허용되지 않은 터보 키 다운로드 주소입니다")
  }
  const assetResponse = await fetchWithTimeout(fetchImpl, assetUrl.href, {
    headers: { Accept: "application/octet-stream" },
  })
  if (!assetResponse.ok) {
    throw new Error(`터보 키 실행 파일을 다운로드하지 못했습니다 (${assetResponse.status})`)
  }
  const buffer = Buffer.from(await assetResponse.arrayBuffer())
  if (!(buffer.length > 0 && buffer.length <= maximumHelperSize)) {
    throw new Error("다운로드한 터보 키 실행 파일의 크기가 올바르지 않습니다")
  }
  const expectedSha256 = digestMatch[1].toLowerCase()
  if (sha256Buffer(buffer) !== expectedSha256) {
    throw new Error("다운로드한 터보 키 실행 파일의 SHA-256이 일치하지 않습니다")
  }
  return {
    buffer,
    sha256: expectedSha256,
    source: release.html_url,
  }
}

export async function installTurboKeyHelper({
  directory,
  appVersion,
  owner = "rubystarashe",
  repo = "nogirem",
  acceptedAt = Date.now(),
  localSourcePath = null,
  fetchImpl = fetch,
}) {
  let downloaded
  if (localSourcePath) {
    const buffer = await readFile(localSourcePath)
    if (buffer.length > maximumHelperSize) {
      throw new Error("터보 키 실행 파일의 크기가 올바르지 않습니다")
    }
    downloaded = {
      buffer,
      sha256: sha256Buffer(buffer),
      source: "local-development-build",
    }
  } else {
    downloaded = await downloadReleaseAsset({
      appVersion,
      owner,
      repo,
      fetchImpl,
    })
  }
  validateTurboKeyExecutable(downloaded.buffer)

  const paths = helperPaths(directory)
  await mkdir(dirname(paths.executablePath), { recursive: true })
  const temporaryPath = `${paths.executablePath}.${process.pid}.${randomUUID()}.tmp`
  await writeFile(temporaryPath, downloaded.buffer)
  await rename(temporaryPath, paths.executablePath)
  await writeJsonAtomic(paths.manifestPath, {
    helperVersion: turboKeyHelperVersion,
    protocolVersion: turboKeyHelperProtocolVersion,
    sha256: downloaded.sha256,
    source: downloaded.source,
    acceptedAt,
    installedAt: Date.now(),
  })

  const installation = await getTurboKeyHelperInstallation(directory)
  if (!installation.installed) {
    throw new Error(installation.reason ?? "터보 키 설치 상태를 확인하지 못했습니다")
  }
  return installation
}
