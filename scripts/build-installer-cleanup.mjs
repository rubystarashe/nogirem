import { spawn } from "node:child_process"
import { copyFile, mkdir, rm } from "node:fs/promises"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const sourceDirectory = join(root, "native", "installer-cleanup")
const targetDirectory = join(sourceDirectory, "target")
const outputDirectory = join(sourceDirectory, "bin")
const buildTarget = "x86_64-pc-windows-msvc"

function run(command, arguments_, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: root,
      shell: false,
      stdio: "inherit",
      windowsHide: true,
      ...options,
    })
    child.once("error", reject)
    child.once("exit", code => {
      if (code === 0) resolve()
      else reject(new Error(`${command} 명령이 종료 코드 ${code}로 실패했습니다`))
    })
  })
}

if (process.platform !== "win32" || process.arch !== "x64") {
  throw new Error("설치 정리 helper는 Windows x64에서만 빌드할 수 있습니다")
}

const builtExecutable = join(
  targetDirectory,
  buildTarget,
  "release",
  "nogirem-installer-cleanup.exe",
)
const packagedExecutable = join(outputDirectory, "installer-cleanup.exe")
await rm(builtExecutable, { force: true })
await rm(packagedExecutable, { force: true })
await run("cargo", [
  "build",
  "--release",
  "--locked",
  "--target",
  buildTarget,
  "--manifest-path",
  join(sourceDirectory, "Cargo.toml"),
], {
  env: { ...process.env, CARGO_TARGET_DIR: targetDirectory },
})
await mkdir(outputDirectory, { recursive: true })
await copyFile(
  builtExecutable,
  packagedExecutable,
)
console.log(`설치 정리 helper 빌드 완료: ${packagedExecutable}`)
