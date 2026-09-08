import assert from "node:assert/strict"
import { createHash } from "node:crypto"
import { mkdir, mkdtemp, readFile, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import test from "node:test"
import {
  applyInstalledDxvk,
  detectDxvkRendererFromLog,
  getDxvkDeploymentStatus,
  getDxvkReleases,
  getInstalledDxvk,
  getLatestDxvkRelease,
} from "../src/dxvk.mjs"

test("DXVK 장치와 스왑체인 초기화 완료 로그를 Vulkan 실행으로 판정한다", () => {
  const result = detectDxvkRendererFromLog([
    "info: Game: Client.exe",
    "info: DXVK: v2.7.1+",
    "info: Creating device:",
    "info: Presenter: Actual swapchain properties:",
  ].join("\n"))

  assert.deepEqual(result, { initialized: true, version: "v2.7.1+" })
})

test("DXVK 헤더만 남은 초기화 실패 로그는 Vulkan 실행으로 판정하지 않는다", () => {
  const result = detectDxvkRendererFromLog([
    "info: Game: Client.exe",
    "info: DXVK: v2.7.1+",
    "info: Process set as DPI aware",
  ].join("\n"))

  assert.deepEqual(result, { initialized: false, version: "v2.7.1+" })
})

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

test("검증된 DXVK를 게임 폴더의 d3d9_dxvk.dll로 적용한다", async () => {
  const root = await mkdtemp(join(tmpdir(), "nogirem-dxvk-apply-"))
  const vulkanDirectory = join(root, "vulkan")
  const targetPath = join(root, "Mabinogi", "d3d9_dxvk.dll")
  const dll = Buffer.from("verified-dxvk")
  const digest = createHash("sha256").update(dll).digest("hex")
  try {
    await mkdir(vulkanDirectory, { recursive: true })
    await writeFile(join(vulkanDirectory, "dxvk-v3.1-d3d9.dll"), dll)
    await writeFile(join(vulkanDirectory, "current.json"), JSON.stringify({
      version: "v3.1",
      fileName: "dxvk-v3.1-d3d9.dll",
      sha256: digest,
    }))
    const installed = await getInstalledDxvk(vulkanDirectory)
    assert.deepEqual(
      await getDxvkDeploymentStatus(installed, targetPath),
      { exists: false, matchesCurrent: false },
    )

    await applyInstalledDxvk(vulkanDirectory, targetPath)

    assert.deepEqual(await readFile(targetPath), dll)
    assert.deepEqual(
      await getDxvkDeploymentStatus(installed, targetPath),
      { exists: true, matchesCurrent: true },
    )
  } finally {
    await rm(root, { recursive: true, force: true })
  }
})

test("관리 창의 확인·설치 완료 상태를 메인 화면에 즉시 전달한다", async () => {
  const [mainSource, preloadSource, appSource] = await Promise.all([
    readFile(new URL("../electron/main.mjs", import.meta.url), "utf8"),
    readFile(new URL("../electron/preload.cjs", import.meta.url), "utf8"),
    readFile(new URL("../web/App.svelte", import.meta.url), "utf8"),
  ])

  assert.match(mainSource, /optimization:dxvk-status-changed/)
  assert.match(
    mainSource,
    /const status = await dxvkRuntimeCheckPromise\s+notifyDxvkRuntimeStatusChanged\(\)\s+return status/,
  )
  assert.match(preloadSource, /onDxvkStatusChanged/)
  assert.match(appSource, /removeDxvkStatusListener/)
  assert.match(appSource, /dxvk: status/)
  assert.match(appSource, /data\?\.dxvk\?\.state === "checking"[\s\S]*dxvk: currentDxvk/)
  assert.match(appSource, /class:checking=\{dxvkLinkState\(\) === "checking"\}/)
  assert.match(appSource, /DXVK 상태 확인 불가/)
  assert.match(appSource, /DXVK 확인 중/)
})
