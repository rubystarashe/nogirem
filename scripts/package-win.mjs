import { execFileSync, spawn } from "node:child_process"
import { existsSync } from "node:fs"
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

function readGithubCliToken() {
  const commands = ["gh"]
  if (process.platform === "win32") {
    const programFiles = process.env.ProgramFiles ?? "C:\\Program Files"
    const installedCommand = join(programFiles, "GitHub CLI", "gh.exe")
    if (existsSync(installedCommand)) commands.unshift(installedCommand)
  }
  for (const command of commands) {
    try {
      const token = execFileSync(command, ["auth", "token"], {
        encoding: "utf8",
        windowsHide: true,
        stdio: ["ignore", "pipe", "ignore"],
      }).trim()
      if (token) return token
    } catch {
    }
  }
  return null
}

function ensureReleaseTag() {
  const status = execFileSync("git", ["status", "--porcelain"], {
    encoding: "utf8",
    windowsHide: true,
  }).trim()
  if (status) {
    throw new Error("GitHub 배포 전에 변경 사항을 커밋해야 합니다")
  }

  const tag = `v${packageInfo.version}`
  const head = execFileSync("git", ["rev-parse", "HEAD"], {
    encoding: "utf8",
    windowsHide: true,
  }).trim()
  let tagCommit
  try {
    tagCommit = execFileSync("git", ["rev-list", "-n", "1", tag], {
      encoding: "utf8",
      windowsHide: true,
      stdio: ["ignore", "pipe", "ignore"],
    }).trim()
  } catch {
    execFileSync("git", ["tag", tag], { windowsHide: true })
    tagCommit = head
  }
  if (tagCommit !== head) {
    throw new Error(`${tag} 태그가 현재 커밋과 일치하지 않습니다`)
  }
  execFileSync("git", ["push", "origin", tag], {
    windowsHide: true,
    stdio: "inherit",
  })
}

if (publishRequested && !process.env.GH_TOKEN) {
  if (process.env.GITHUB_TOKEN) {
    process.env.GH_TOKEN = process.env.GITHUB_TOKEN
  } else {
    const githubCliToken = readGithubCliToken()
    if (!githubCliToken) {
      throw new Error("GitHub CLI에서 gh auth login을 완료하거나 GH_TOKEN을 설정해야 합니다")
    }
    process.env.GH_TOKEN = githubCliToken
  }
}
if (publishRequested) ensureReleaseTag()

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
