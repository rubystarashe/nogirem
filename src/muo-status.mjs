import { createDecipheriv, createHash } from 'node:crypto'
import { readdir, readFile, stat } from 'node:fs/promises'
import { homedir } from 'node:os'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import { inflateSync } from 'node:zlib'

const TARGET_ATTRIBUTE = 'DummyCharRenderModeFPS'
const MUO2_KEY_SEED_HEX = '7a145a9f319947b9faaf2488b8bbbe63'
const MUO2_KEY_SUFFIX = 'MUO_FILE_KEY_V1'

function crc32(bytes) {
  let crc = 0xffffffff
  for (const byte of bytes) {
    crc ^= byte
    for (let bit = 0; bit < 8; bit += 1) {
      crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0)
    }
  }
  return (crc ^ 0xffffffff) >>> 0
}

function assert(condition, message) {
  if (!condition) throw new Error(message)
}

function utf16LeWithTerminator(text) {
  return Buffer.concat([Buffer.from(text, 'utf16le'), Buffer.alloc(2)])
}

function verifyInnerCrc(innerText, storedCrc) {
  const actualCrc = crc32(utf16LeWithTerminator(innerText))
  assert(actualCrc === storedCrc,
    `CRC32 불일치: 저장=0x${storedCrc.toString(16)}, 계산=0x${actualCrc.toString(16)}`)
}

function decodeInnerPayload(innerText) {
  const separator = innerText.indexOf(';')
  assert(separator > 0, 'MUO 내부 헤더가 올바르지 않습니다.')

  const expectedSizeText = innerText.slice(0, separator)
  const compressedHex = innerText.slice(separator + 1)
  assert(/^\d+$/.test(expectedSizeText), 'XML 크기 필드를 읽을 수 없습니다.')
  assert(compressedHex.length > 0 && compressedHex.length % 2 === 0
    && /^[0-9a-f]+$/.test(compressedHex), '압축 데이터가 유효한 소문자 16진수가 아닙니다.')

  const xmlBytes = inflateSync(Buffer.from(compressedHex, 'hex'))
  const expectedSize = Number.parseInt(expectedSizeText, 10)
  assert(xmlBytes.length === expectedSize,
    `XML 크기 불일치: 예상=${expectedSize}, 실제=${xmlBytes.length}`)
  assert(xmlBytes.length >= 2 && xmlBytes.at(-2) === 0 && xmlBytes.at(-1) === 0,
    'UTF-16LE 종료 문자가 없습니다.')

  return xmlBytes.subarray(0, -2).toString('utf16le').replace(/^\uFEFF/, '')
}

function decodeLegacyMuo(bytes) {
  assert(bytes.length >= 8, 'MUO 파일이 너무 짧습니다.')
  const storedCrc = bytes.readUInt32LE(0)
  const characterCount = bytes.readUInt32LE(4)
  assert(bytes.length === 8 + characterCount * 2,
    `MUO 길이 불일치: 헤더 문자 수=${characterCount}, 파일 크기=${bytes.length}`)

  const innerText = bytes.subarray(8).toString('utf16le')
  verifyInnerCrc(innerText, storedCrc)
  return { format: 'legacyMuo', xmlText: decodeInnerPayload(innerText) }
}

function decodeMuo2(bytes) {
  assert(bytes.length >= 0x24, 'MUO2 파일이 너무 짧습니다.')
  const version = bytes.readUInt16LE(0x04)
  const flags = bytes.readUInt16LE(0x06)
  const storedCount = bytes.readUInt32LE(0x08)
  const storedCrc = bytes.readUInt32LE(0x0c)
  const iv = bytes.subarray(0x10, 0x20)
  const repeatedCount = bytes.readUInt32LE(0x20)

  assert(version === 1, `지원하지 않는 MUO2 버전: ${version}`)
  assert(storedCount === repeatedCount && bytes.length === 0x24 + storedCount * 2,
    'MUO2 문자열 길이와 파일 크기가 일치하지 않습니다.')

  const storedText = bytes.subarray(0x24).toString('utf16le')
  let innerText = storedText
  if (flags & 0x02) {
    const key = createHash('sha256')
      .update(Buffer.from(MUO2_KEY_SEED_HEX, 'hex'))
      .update(Buffer.from(MUO2_KEY_SUFFIX, 'utf8'))
      .digest()
      .subarray(0, 16)
    const decipher = createDecipheriv('aes-128-cbc', key, iv)
    innerText = Buffer.concat([
      decipher.update(Buffer.from(storedText, 'base64')),
      decipher.final(),
    ]).toString('utf8')
  }

  verifyInnerCrc(innerText, storedCrc)
  return { format: 'muo2', xmlText: decodeInnerPayload(innerText) }
}

export function decodeMuo(bytes) {
  return bytes.subarray(0, 4).toString('ascii') === 'MUO2'
    ? decodeMuo2(bytes)
    : decodeLegacyMuo(bytes)
}

function extractAttribute(xmlText, attribute = TARGET_ATTRIBUTE) {
  const escaped = attribute.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
  const match = xmlText.match(new RegExp(`\\b${escaped}\\s*=\\s*(["'])(.*?)\\1`, 'i'))
  assert(match, `${attribute} 속성을 찾을 수 없습니다.`)
  return match[2]
}

export async function getLatestMuoStatus(directory = path.join(homedir(), 'Documents', '마비노기', '설정')) {
  const entries = await readdir(directory, { withFileTypes: true })
  const candidates = await Promise.all(entries
    .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith('.muo'))
    .map(async (entry) => {
      const filePath = path.join(directory, entry.name)
      const info = await stat(filePath)
      return { filePath, fileName: entry.name, mtimeMs: info.mtimeMs, modifiedAt: info.mtime.toISOString() }
    }))

  assert(candidates.length > 0, `.muo 파일이 없습니다: ${directory}`)
  candidates.sort((a, b) => b.mtimeMs - a.mtimeMs || b.fileName.localeCompare(a.fileName))

  const latest = candidates[0]
  const decoded = decodeMuo(await readFile(latest.filePath))
  const value = extractAttribute(decoded.xmlText)
  return {
    applied: value === '-1',
    attribute: TARGET_ATTRIBUTE,
    value,
    format: decoded.format,
    fileName: latest.fileName,
    filePath: latest.filePath,
    modifiedAt: latest.modifiedAt,
  }
}

async function main() {
  const args = process.argv.slice(2)
  const json = args.includes('--json')
  const dirIndex = args.indexOf('--dir')
  const directory = dirIndex >= 0 ? args[dirIndex + 1] : undefined
  assert(dirIndex < 0 || directory, '--dir 다음에 폴더 경로가 필요합니다.')

  const result = await getLatestMuoStatus(directory)
  if (json) {
    console.log(JSON.stringify(result, null, 2))
  } else {
    console.log(`최신 MUO: ${result.filePath}`)
    console.log(`형식: ${result.format}`)
    console.log(`${result.attribute}="${result.value}"`)
    console.log(`-1 적용 여부: ${result.applied ? '적용됨' : '적용 안 됨'}`)
  }
}

const isMain = process.argv[1]
  && path.resolve(process.argv[1]) === path.resolve(fileURLToPath(import.meta.url))
if (isMain) {
  main().catch((error) => {
    console.error(`MUO 확인 실패: ${error.message}`)
    process.exitCode = 2
  })
}
