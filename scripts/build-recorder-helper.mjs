import { spawn } from "node:child_process"
import { createHash } from "node:crypto"
import { existsSync } from "node:fs"
import { copyFile, mkdir, rm, writeFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const sourceDirectory = join(root, "native", "recorder-helper")
const buildDirectory = join(sourceDirectory, "build")
const generatedDirectory = join(buildDirectory, "generated")
const outputDirectory = join(sourceDirectory, "bin")
const cppWinrtVersion = "2.0.240111.5"
const cppWinrtSha256 = "4677321a12bef84efe46e8f3145352f1091effa562c8b0a46799b2f7de13779b"
const cppWinrtDirectory = join(tmpdir(), `cppwinrt-${cppWinrtVersion}`)
const cppWinrtArchive = `${cppWinrtDirectory}.zip`
const cppWinrtExecutable = join(cppWinrtDirectory, "bin", "cppwinrt.exe")

function run(command, arguments_) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: root,
      shell: false,
      stdio: "inherit",
      windowsHide: true,
    })
    child.once("error", reject)
    child.once("exit", code => {
      if (code === 0) resolve()
      else reject(new Error(`${command} 명령이 종료 코드 ${code}로 실패했습니다`))
    })
  })
}

if (process.platform !== "win32") {
  throw new Error("블랙박스 녹화 helper는 Windows x64에서만 빌드할 수 있습니다")
}

if (!existsSync(cppWinrtExecutable)) {
  const response = await fetch(
    `https://www.nuget.org/api/v2/package/Microsoft.Windows.CppWinRT/${cppWinrtVersion}`,
  )
  if (!response.ok) {
    throw new Error(`공식 C++/WinRT 빌드 도구 다운로드 실패 (${response.status})`)
  }
  const archive = Buffer.from(await response.arrayBuffer())
  const sha256 = createHash("sha256").update(archive).digest("hex")
  if (sha256 !== cppWinrtSha256) {
    throw new Error("공식 C++/WinRT 빌드 도구의 SHA-256이 일치하지 않습니다")
  }
  await writeFile(cppWinrtArchive, archive)
  await run("powershell.exe", [
    "-NoProfile",
    "-NonInteractive",
    "-Command",
    `Expand-Archive -LiteralPath '${cppWinrtArchive.replaceAll("'", "''")}' -DestinationPath '${cppWinrtDirectory.replaceAll("'", "''")}' -Force`,
  ])
}

await rm(generatedDirectory, { recursive: true, force: true })
await mkdir(generatedDirectory, { recursive: true })
await run(cppWinrtExecutable, [
  "-input",
  "sdk",
  "-output",
  generatedDirectory,
])
await run("cmake", [
  "-S",
  sourceDirectory,
  "-B",
  buildDirectory,
  "-G",
  "Visual Studio 16 2019",
  "-A",
  "x64",
])
await run("cmake", ["--build", buildDirectory, "--config", "Release"])
await mkdir(outputDirectory, { recursive: true })
await copyFile(
  join(buildDirectory, "Release", "recorder-helper.exe"),
  join(outputDirectory, "recorder-helper.exe"),
)
console.log(`블랙박스 녹화 helper 빌드 완료: ${join(outputDirectory, "recorder-helper.exe")}`)
