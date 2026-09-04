import { createHash, randomUUID } from "node:crypto"
import { execFile } from "node:child_process"
import {
  mkdir,
  readFile,
  rename,
  rm,
  unlink,
  writeFile,
} from "node:fs/promises"
import { basename, join } from "node:path"
import { tmpdir } from "node:os"
import { promisify } from "node:util"

const execFileAsync = promisify(execFile)
const latestReleaseUrl = "https://api.github.com/repos/doitsujin/dxvk/releases/latest"
const releasesUrl = "https://api.github.com/repos/doitsujin/dxvk/releases?per_page=100"
const maximumArchiveBytes = 64 * 1024 * 1024

function sha256(buffer) {
  return createHash("sha256").update(buffer).digest("hex")
}

function normalizeVersion(tagName) {
  if (!/^v\d+(?:\.\d+){1,3}$/.test(tagName)) {
    throw new Error("DXVK 릴리즈 버전 형식이 올바르지 않습니다")
  }
  return tagName
}

function validateDll(buffer) {
  if (buffer.length < 256 || buffer.toString("ascii", 0, 2) !== "MZ") {
    throw new Error("추출한 DXVK 파일이 Windows DLL 형식이 아닙니다")
  }
  const peOffset = buffer.readUInt32LE(0x3c)
  if (
    peOffset + 6 > buffer.length
    || buffer.toString("ascii", peOffset, peOffset + 4) !== "PE\u0000\u0000"
    || buffer.readUInt16LE(peOffset + 4) !== 0x8664
  ) {
    throw new Error("추출한 DXVK 파일이 x64 DLL 형식이 아닙니다")
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

function parseRelease(release) {
  if (release.draft || release.prerelease) {
    throw new Error("DXVK 정식 릴리즈가 아닙니다")
  }
  const version = normalizeVersion(release.tag_name)
  const asset = release.assets?.find(item => (
    /^dxvk-[\d.]+\.tar\.gz$/.test(item.name)
    && item.state === "uploaded"
  ))
  if (!asset?.browser_download_url) {
    throw new Error("DXVK 릴리즈 압축 파일을 찾지 못했습니다")
  }
  const digest = /^sha256:([a-f0-9]{64})$/i.exec(asset.digest ?? "")
  if (!digest) {
    throw new Error("DXVK 릴리즈 SHA-256 정보가 없습니다")
  }
  return {
    version,
    releaseUrl: release.html_url,
    downloadUrl: asset.browser_download_url,
    archiveName: asset.name,
    archiveSize: asset.size,
    archiveSha256: digest[1].toLowerCase(),
    publishedAt: release.published_at,
  }
}

async function fetchReleaseJson(url, fetchImpl, failureMessage) {
  const response = await fetchImpl(url, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "mabinogi-rem-booster",
      "X-GitHub-Api-Version": "2022-11-28",
    },
  })
  if (!response.ok) throw new Error(`${failureMessage} (${response.status})`)
  return response.json()
}

export async function getLatestDxvkRelease(fetchImpl = globalThis.fetch) {
  const release = await fetchReleaseJson(
    latestReleaseUrl,
    fetchImpl,
    "DXVK 최신 릴리즈 조회 실패",
  )
  try {
    return parseRelease(release)
  } catch {
    throw new Error("DXVK 최신 정식 릴리즈를 찾지 못했습니다")
  }
}

export async function getDxvkReleases(fetchImpl = globalThis.fetch) {
  const response = await fetchReleaseJson(
    releasesUrl,
    fetchImpl,
    "DXVK 릴리즈 목록 조회 실패",
  )
  if (!Array.isArray(response)) throw new Error("DXVK 릴리즈 목록 형식이 올바르지 않습니다")
  const releases = response.flatMap(release => {
    try {
      return [parseRelease(release)]
    } catch {
      return []
    }
  })
  if (releases.length === 0) {
    throw new Error("SHA-256 검증 가능한 DXVK 정식 릴리즈가 없습니다")
  }
  return releases
}

export async function getInstalledDxvk(vulkanDirectory) {
  const currentPath = join(vulkanDirectory, "current.json")
  const current = await readJson(currentPath)
  if (!current?.fileName || basename(current.fileName) !== current.fileName) {
    return { installed: false, current: null, integrity: null }
  }
  try {
    const dll = await readFile(join(vulkanDirectory, current.fileName))
    const actualSha256 = sha256(dll)
    return {
      installed: true,
      current,
      integrity: actualSha256 === current.sha256,
      actualSha256,
    }
  } catch (error) {
    if (error?.code === "ENOENT") {
      return { installed: false, current, integrity: false }
    }
    throw error
  }
}

async function installDxvkRelease(vulkanDirectory, release, fetchImpl) {
  const installed = await getInstalledDxvk(vulkanDirectory)
  if (
    installed.installed
    && installed.integrity
    && installed.current.version === release.version
    && installed.current.archiveSha256 === release.archiveSha256
  ) {
    return { release, installed, updated: false }
  }

  const workDirectory = join(tmpdir(), `nogirem-dxvk-${randomUUID()}`)
  const archivePath = join(workDirectory, release.archiveName)
  await mkdir(workDirectory, { recursive: true })
  try {
    const response = await fetchImpl(release.downloadUrl, {
      headers: { "User-Agent": "mabinogi-rem-booster" },
    })
    if (!response.ok) {
      throw new Error(`DXVK 압축 파일 다운로드 실패 (${response.status})`)
    }
    const declaredLength = Number(response.headers.get("content-length"))
    if (declaredLength > maximumArchiveBytes) {
      throw new Error("DXVK 압축 파일이 허용 크기를 초과했습니다")
    }
    const archive = Buffer.from(await response.arrayBuffer())
    if (archive.length === 0 || archive.length > maximumArchiveBytes) {
      throw new Error("DXVK 압축 파일 크기가 올바르지 않습니다")
    }
    const archiveSha256 = sha256(archive)
    if (archiveSha256 !== release.archiveSha256) {
      throw new Error("DXVK 압축 파일 SHA-256 검증에 실패했습니다")
    }
    await writeFile(archivePath, archive)

    const { stdout } = await execFileAsync(
      "tar.exe",
      ["-tzf", archivePath],
      { windowsHide: true, maxBuffer: 8 * 1024 * 1024 },
    )
    const dllEntry = stdout
      .split(/\r?\n/)
      .find(entry => /^[^/]+\/x64\/d3d9\.dll$/i.test(entry))
    if (!dllEntry) throw new Error("DXVK 압축 파일에서 x64/d3d9.dll을 찾지 못했습니다")

    await execFileAsync(
      "tar.exe",
      ["-xzf", archivePath, "-C", workDirectory, dllEntry],
      { windowsHide: true, timeout: 120000 },
    )
    const extractedPath = join(workDirectory, ...dllEntry.split("/"))
    const dll = await readFile(extractedPath)
    validateDll(dll)

    await mkdir(vulkanDirectory, { recursive: true })
    const fileName = `dxvk-${release.version}-d3d9.dll`
    const destinationPath = join(vulkanDirectory, fileName)
    const temporaryDllPath = `${destinationPath}.${randomUUID()}.tmp`
    await writeFile(temporaryDllPath, dll)
    await unlink(destinationPath).catch(error => {
      if (error?.code !== "ENOENT") throw error
    })
    await rename(temporaryDllPath, destinationPath)

    const current = {
      version: release.version,
      fileName,
      sha256: sha256(dll),
      archiveSha256: release.archiveSha256,
      downloadUrl: release.downloadUrl,
      releaseUrl: release.releaseUrl,
      publishedAt: release.publishedAt,
      installedAt: new Date().toISOString(),
    }
    const currentPath = join(vulkanDirectory, "current.json")
    const temporaryCurrentPath = `${currentPath}.${randomUUID()}.tmp`
    await writeFile(temporaryCurrentPath, JSON.stringify(current, null, 2), "utf8")
    await rename(temporaryCurrentPath, currentPath)

    const previousFileName = installed.current?.fileName
    if (
      previousFileName
      && previousFileName !== fileName
      && basename(previousFileName) === previousFileName
      && /^dxvk-v[\d.]+-d3d9\.dll$/i.test(previousFileName)
    ) {
      await unlink(join(vulkanDirectory, previousFileName)).catch(error => {
        if (error?.code !== "ENOENT") throw error
      })
    }
    return {
      release,
      installed: await getInstalledDxvk(vulkanDirectory),
      updated: true,
    }
  } finally {
    await rm(workDirectory, { recursive: true, force: true })
  }
}

export async function installLatestDxvk(vulkanDirectory, fetchImpl = globalThis.fetch) {
  const release = await getLatestDxvkRelease(fetchImpl)
  return installDxvkRelease(vulkanDirectory, release, fetchImpl)
}

export async function installDxvkVersion(
  vulkanDirectory,
  version,
  fetchImpl = globalThis.fetch,
) {
  const normalizedVersion = normalizeVersion(version)
  const releases = await getDxvkReleases(fetchImpl)
  const release = releases.find(item => item.version === normalizedVersion)
  if (!release) throw new Error("선택한 DXVK 정식 릴리즈를 찾지 못했습니다")
  return installDxvkRelease(vulkanDirectory, release, fetchImpl)
}
