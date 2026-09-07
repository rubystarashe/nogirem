import { open, readdir, stat } from "node:fs/promises"
import { homedir } from "node:os"
import { extname, join, relative } from "node:path"
import AdmZip from "adm-zip"

const maximumFileBytes = 5 * 1024 * 1024
const maximumBundleSourceBytes = 25 * 1024 * 1024
const allowedExtensions = new Set([".json", ".lock", ".log", ".txt", ".yml", ".yaml"])
const excludedDirectories = new Set([
  "blob_storage",
  "cache",
  "code cache",
  "crashpad",
  "dawncache",
  "gpucache",
  "clips",
  "ring",
  "bin",
  "session storage",
  "local storage",
])

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")
}

function redactIpv4(value) {
  return value.replace(/\b(?:\d{1,3}\.){3}\d{1,3}\b/g, candidate => {
    const octets = candidate.split(".").map(Number)
    return octets.every(octet => octet >= 0 && octet <= 255)
      ? "%IP_ADDRESS%"
      : candidate
  })
}

export function redactDiagnosticText(value, {
  privatePaths = [],
  computerName = process.env.COMPUTERNAME ?? "",
} = {}) {
  let result = String(value)
  const paths = [...new Set([
    ...privatePaths,
    homedir(),
    process.env.USERPROFILE,
    process.env.APPDATA,
    process.env.LOCALAPPDATA,
  ].filter(Boolean))].sort((left, right) => right.length - left.length)
  for (const path of paths) {
    const jsonEscapedPath = path.replaceAll("\\", "\\\\")
    result = result.replace(
      new RegExp(escapeRegExp(jsonEscapedPath), "gi"),
      "%USER_DATA%",
    )
    result = result.replace(new RegExp(escapeRegExp(path), "gi"), "%USER_DATA%")
  }
  result = result.replace(/(?:[A-Za-z]:\\)?Users\\[^\\/\s"]+/gi, "%USERPROFILE%")
  result = result.replace(
    /(?:[A-Za-z]:\\\\)?Users\\\\[^\\/\s"]+/gi,
    "%USERPROFILE%",
  )
  if (computerName) {
    result = result.replace(
      new RegExp(`\\\\\\\\${escapeRegExp(computerName)}(?=\\\\|\\s|$)`, "gi"),
      "\\\\%COMPUTERNAME%",
    )
  }
  result = redactIpv4(result)
  result = result.replace(
    /\b[A-Z0-9._%+-]+@[A-Z0-9.-]+\.[A-Z]{2,}\b/gi,
    "%EMAIL_ADDRESS%",
  )
  result = result.replace(
    /("(?:access_?token|refresh_?token|authorization|password|cookie|secret)"\s*:\s*)"[^"]*"/gi,
    "$1\"%REDACTED%\"",
  )
  result = result.replace(/\bBearer\s+[A-Za-z0-9._~+/=-]+/gi, "Bearer %REDACTED%")
  result = result.replace(/\b(?:ghp|github_pat)_[A-Za-z0-9_]+/g, "%REDACTED_TOKEN%")
  result = result.replace(
    /([?&](?:token|key|secret|signature|sig)=)[^&#\s]+/gi,
    "$1%REDACTED%",
  )
  return result
}

function decodeDiagnosticFile(buffer) {
  if (buffer.length >= 2 && buffer[0] === 0xff && buffer[1] === 0xfe) {
    return buffer.subarray(2).toString("utf16le")
  }
  return buffer.toString("utf8")
}

async function readDiagnosticTail(path, size) {
  const bytesToRead = Math.min(size, maximumFileBytes)
  const buffer = Buffer.alloc(bytesToRead)
  const handle = await open(path, "r")
  try {
    const { bytesRead } = await handle.read(
      buffer,
      0,
      bytesToRead,
      Math.max(0, size - bytesToRead),
    )
    return {
      buffer: buffer.subarray(0, bytesRead),
      truncated: size > bytesToRead,
    }
  } finally {
    await handle.close()
  }
}

export async function collectDiagnosticEntries(userDataPath, redactionOptions = {}) {
  const entries = []
  let sourceBytes = 0

  async function walk(directory, depth = 0) {
    if (depth > 6 || sourceBytes >= maximumBundleSourceBytes) return
    let directoryEntries
    try {
      directoryEntries = await readdir(directory, { withFileTypes: true })
    } catch {
      return
    }
    directoryEntries.sort((left, right) => left.name.localeCompare(right.name))
    for (const entry of directoryEntries) {
      if (sourceBytes >= maximumBundleSourceBytes) break
      const path = join(directory, entry.name)
      if (entry.isDirectory()) {
        if (!excludedDirectories.has(entry.name.toLowerCase())) {
          await walk(path, depth + 1)
        }
        continue
      }
      if (!entry.isFile() || !allowedExtensions.has(extname(entry.name).toLowerCase())) {
        continue
      }
      try {
        const fileInfo = await stat(path)
        const remaining = maximumBundleSourceBytes - sourceBytes
        if (remaining <= 0) break
        const truncatedByBudget = Number(fileInfo.size) > remaining
        const { buffer, truncated } = await readDiagnosticTail(
          path,
          Math.min(Number(fileInfo.size), remaining),
        )
        sourceBytes += buffer.length
        const wasTruncated = truncated || truncatedByBudget
        const prefix = wasTruncated
          ? `[파일이 커서 최근 ${buffer.length}바이트만 포함됨]\n`
          : ""
        const text = prefix + decodeDiagnosticFile(buffer)
        entries.push({
          name: relative(userDataPath, path).replaceAll("\\", "/"),
          content: redactDiagnosticText(text, redactionOptions),
          originalBytes: Number(fileInfo.size),
          includedBytes: buffer.length,
          truncated: wasTruncated,
        })
      } catch {
      }
    }
  }

  await walk(userDataPath)
  return entries
}

export async function createDiagnosticBundle({
  outputPath,
  userDataPath,
  diagnostics,
  redactionOptions = {},
}) {
  const entries = await collectDiagnosticEntries(userDataPath, {
    ...redactionOptions,
    privatePaths: [
      userDataPath,
      ...(redactionOptions.privatePaths ?? []),
    ],
  })
  const zip = new AdmZip()
  const sanitizedDiagnostics = redactDiagnosticText(
    JSON.stringify(diagnostics, null, 2),
    {
      ...redactionOptions,
      privatePaths: [
        userDataPath,
        ...(redactionOptions.privatePaths ?? []),
      ],
    },
  )
  zip.addFile("diagnostics.json", Buffer.from(sanitizedDiagnostics))
  for (const entry of entries) {
    zip.addFile(`files/${entry.name}`, Buffer.from(entry.content))
  }
  zip.addFile("included-files.json", Buffer.from(JSON.stringify(
    entries.map(entry => ({
      name: entry.name,
      originalBytes: entry.originalBytes,
      includedBytes: entry.includedBytes,
      truncated: entry.truncated,
    })),
    null,
    2,
  )))
  await new Promise((resolve, reject) => {
    zip.writeZip(outputPath, error => {
      if (error) reject(error)
      else resolve()
    })
  })
  return {
    outputPath,
    fileCount: entries.length + 2,
  }
}
