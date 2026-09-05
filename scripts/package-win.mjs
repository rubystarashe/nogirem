import { spawn } from "node:child_process"
import { copyFile, mkdir, readFile } from "node:fs/promises"
import { tmpdir } from "node:os"
import { dirname, join } from "node:path"
import { fileURLToPath } from "node:url"

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const packageInfo = JSON.parse(await readFile(join(root, "package.json"), "utf8"))
const temporaryOutput = join(tmpdir(), "nogirem-builder-output")
const releaseOutput = join(root, "release")
const installerName = `nogirem-setup-${packageInfo.version}.exe`
const publishRequested = process.argv.includes("--publish")

if (publishRequested && !process.env.GH_TOKEN) {
  if (process.env.GITHUB_TOKEN) {
    process.env.GH_TOKEN = process.env.GITHUB_TOKEN
  } else {
    throw new Error("GitHub 배포에는 GH_TOKEN 또는 GITHUB_TOKEN 환경 변수가 필요합니다")
  }
}

function run(command, arguments_) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, arguments_, {
      cwd: root,
      shell: process.platform === "win32",
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

await run("npm", ["run", "native:radeon"])
await run("npm", ["run", "app:build"])
const builderArguments = [
  "electron-builder",
  "--win",
  "nsis",
  "--x64",
  `--config.directories.output=${temporaryOutput}`,
]
if (publishRequested) builderArguments.push("--publish", "always")
await run("npx", builderArguments)
await mkdir(releaseOutput, { recursive: true })
await Promise.all([
  copyFile(
    join(temporaryOutput, installerName),
    join(releaseOutput, installerName),
  ),
  copyFile(
    join(temporaryOutput, `${installerName}.blockmap`),
    join(releaseOutput, `${installerName}.blockmap`),
  ),
  copyFile(
    join(temporaryOutput, "latest.yml"),
    join(releaseOutput, "latest.yml"),
  ),
])
console.log(`패키징 완료: ${join(releaseOutput, installerName)}`)
