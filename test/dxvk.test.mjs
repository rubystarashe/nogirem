import assert from "node:assert/strict"
import { createHash } from "node:crypto"
import { mkdtemp, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"
import {
  getDxvkReleases,
  getInstalledDxvk,
  getLatestDxvkRelease,
} from "../src/dxvk.mjs"

test("DXVK 최신 정식 릴리즈의 압축 파일과 SHA-256을 해석한다", async () => {
  const fetchImpl = async () => ({
    ok: true,
    json: async () => ({
      tag_name: "v3.1",
      draft: false,
      prerelease: false,
      html_url: "https://github.com/doitsujin/dxvk/releases/tag/v3.1",
      published_at: "2026-08-28T16:23:29Z",
      assets: [{
        name: "dxvk-3.1.tar.gz",
        state: "uploaded",
        size: 18056721,
        digest: `sha256:${"a".repeat(64)}`,
        browser_download_url: "https://example.com/dxvk-3.1.tar.gz",
      }],
    }),
  })

  const release = await getLatestDxvkRelease(fetchImpl)
  assert.equal(release.version, "v3.1")
  assert.equal(release.archiveSha256, "a".repeat(64))
  assert.equal(release.archiveName, "dxvk-3.1.tar.gz")
})

test("사전 릴리즈는 최신 정식 DXVK로 허용하지 않는다", async () => {
  const fetchImpl = async () => ({
    ok: true,
    json: async () => ({
      tag_name: "v3.2",
      draft: false,
      prerelease: true,
      assets: [],
    }),
  })

  await assert.rejects(
    getLatestDxvkRelease(fetchImpl),
    /최신 정식 릴리즈/,
  )
})

test("검증 가능한 이전 DXVK 정식 릴리즈 목록을 반환한다", async () => {
  const release = (version, digest = "b".repeat(64)) => ({
    tag_name: version,
    draft: false,
    prerelease: false,
    html_url: `https://github.com/doitsujin/dxvk/releases/tag/${version}`,
    published_at: "2026-01-01T00:00:00Z",
    assets: [{
      name: `dxvk-${version.slice(1)}.tar.gz`,
      state: "uploaded",
      size: 100,
      digest: digest ? `sha256:${digest}` : null,
      browser_download_url: `https://example.com/dxvk-${version}.tar.gz`,
    }],
  })
  const fetchImpl = async () => ({
    ok: true,
    json: async () => [
      release("v3.1"),
      release("v3.0"),
      { ...release("v3.2"), prerelease: true },
      release("v2.7", null),
    ],
  })

  const releases = await getDxvkReleases(fetchImpl)
  assert.deepEqual(releases.map(item => item.version), ["v3.1", "v3.0"])
})

test("저장된 DXVK DLL의 SHA-256 무결성을 확인한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-dxvk-status-"))
  const dll = Buffer.from("dxvk-test")
  const digest = createHash("sha256").update(dll).digest("hex")
  try {
    await writeFile(join(directory, "dxvk-v3.1-d3d9.dll"), dll)
    await writeFile(join(directory, "current.json"), JSON.stringify({
      version: "v3.1",
      fileName: "dxvk-v3.1-d3d9.dll",
      sha256: digest,
    }))
    const installed = await getInstalledDxvk(directory)
    assert.equal(installed.installed, true)
    assert.equal(installed.integrity, true)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
