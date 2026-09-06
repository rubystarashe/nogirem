import assert from "node:assert/strict"
import { createHash } from "node:crypto"
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"
import {
  getTurboKeyHelperInstallation,
  installTurboKeyHelper,
  removeTurboKeyHelper,
  turboKeyHelperAssetName,
  validateTurboKeyExecutable,
} from "../src/turbo-key-installer.mjs"

function createX64Executable() {
  const buffer = Buffer.alloc(512)
  buffer.writeUInt16LE(0x5a4d, 0)
  buffer.writeUInt32LE(0x80, 0x3c)
  buffer.writeUInt32LE(0x00004550, 0x80)
  buffer.writeUInt16LE(0x8664, 0x84)
  return buffer
}

test("터보 키 설치 파일은 Windows x64 PE만 허용한다", () => {
  assert.equal(validateTurboKeyExecutable(createX64Executable()), true)
  assert.throws(() => validateTurboKeyExecutable(Buffer.alloc(512)), /MZ 헤더/)
})

test("개발 빌드 helper를 AppData 형식으로 설치하고 변조를 감지한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-turbo-install-"))
  const sourcePath = join(directory, "source.exe")
  try {
    await writeFile(sourcePath, createX64Executable())
    const installed = await installTurboKeyHelper({
      directory,
      appVersion: "0.2.2",
      localSourcePath: sourcePath,
      acceptedAt: 1234,
    })
    assert.equal(installed.installed, true)
    assert.equal(installed.acceptedAt, 1234)

    await writeFile(installed.executablePath, Buffer.concat([
      await readFile(installed.executablePath),
      Buffer.from([1]),
    ]))
    const tampered = await getTurboKeyHelperInstallation(directory)
    assert.equal(tampered.installed, false)
    assert.match(tampered.reason, /무결성/)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("정식 GitHub Release의 고정 자산과 SHA-256만 설치한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-turbo-release-"))
  const executable = createX64Executable()
  const digest = createHash("sha256").update(executable).digest("hex")
  const fetchImpl = async url => {
    if (String(url).includes("/releases/tags/")) {
      return new Response(JSON.stringify({
        draft: false,
        prerelease: false,
        html_url: "https://github.com/rubystarashe/nogirem/releases/tag/v0.2.2",
        assets: [{
          name: turboKeyHelperAssetName,
          state: "uploaded",
          size: executable.length,
          digest: `sha256:${digest}`,
          browser_download_url: `https://github.com/rubystarashe/nogirem/releases/download/v0.2.2/${turboKeyHelperAssetName}`,
        }],
      }), { status: 200 })
    }
    return new Response(executable, { status: 200 })
  }

  try {
    const installed = await installTurboKeyHelper({
      directory,
      appVersion: "0.2.2",
      fetchImpl,
    })
    assert.equal(installed.installed, true)
    assert.equal(installed.sha256, digest)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("설치된 터보 키 helper와 manifest를 함께 제거한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-turbo-remove-"))
  const sourcePath = join(directory, "source.exe")
  try {
    await writeFile(sourcePath, createX64Executable())
    await installTurboKeyHelper({
      directory,
      appVersion: "0.2.2",
      localSourcePath: sourcePath,
    })
    const removed = await removeTurboKeyHelper(directory)
    assert.equal(removed.installed, false)
    assert.equal(removed.reason, null)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
