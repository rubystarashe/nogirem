import { spawn } from "node:child_process"
import { copyFile, mkdir } from "node:fs/promises"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const sourceDirectory = join(root, "native", "radeon-helper")
const buildDirectory = join(sourceDirectory, "build")
const outputDirectory = join(sourceDirectory, "bin")

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
  throw new Error("Radeon helper는 Windows x64에서만 빌드할 수 있습니다")
}

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
  join(buildDirectory, "Release", "radeon-helper.exe"),
  join(outputDirectory, "radeon-helper.exe"),
)
console.log(`Radeon helper 빌드 완료: ${join(outputDirectory, "radeon-helper.exe")}`)
