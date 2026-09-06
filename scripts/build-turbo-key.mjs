import { spawn } from "node:child_process"
import { copyFile, mkdir } from "node:fs/promises"
import { existsSync } from "node:fs"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const sourceDirectory = join(root, "native", "turbo-key")
const targetDirectory = join(sourceDirectory, "target")
const outputDirectory = join(sourceDirectory, "bin")
const cargoExecutable = process.platform === "win32"
  ? [
      process.env.CARGO_HOME && join(process.env.CARGO_HOME, "bin", "cargo.exe"),
      process.env.USERPROFILE && join(process.env.USERPROFILE, ".cargo", "bin", "cargo.exe"),
      process.env.USERPROFILE && join(
        process.env.USERPROFILE,
        ".rustup",
        "toolchains",
        "stable-x86_64-pc-windows-msvc",
        "bin",
        "cargo.exe",
      ),
      "cargo",
    ].find(candidate => candidate && (candidate === "cargo" || existsSync(candidate)))
  : "cargo"

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
  throw new Error("터보 키 helper는 Windows x64에서만 빌드할 수 있습니다")
}

await run(cargoExecutable, [
  "build",
  "--release",
  "--target",
  "x86_64-pc-windows-msvc",
  "--manifest-path",
  join(sourceDirectory, "Cargo.toml"),
  "--target-dir",
  targetDirectory,
])
await mkdir(outputDirectory, { recursive: true })
await copyFile(
  join(targetDirectory, "x86_64-pc-windows-msvc", "release", "turbo-key-helper.exe"),
  join(outputDirectory, "turbo-key-helper.exe"),
)
console.log(`터보 키 helper 빌드 완료: ${join(outputDirectory, "turbo-key-helper.exe")}`)
