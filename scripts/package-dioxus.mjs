import { spawn } from 'node:child_process'
import { existsSync } from 'node:fs'
import { cp, copyFile, mkdir, readFile, writeFile, readdir, stat } from 'node:fs/promises'
import { dirname, join, relative } from 'node:path'
import { fileURLToPath } from 'node:url'
import { turboKeyHelperAssetName } from '../src/turbo-key-installer.mjs'
import { createTransport } from '../service/transport.mjs'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
if (process.platform !== 'win32' || process.arch !== 'x64') throw new Error('Windows x64 packaging is required')
if (process.argv.includes('--publish')) throw new Error('Automatic publishing is disabled. Prepare and verify the signed release bundle before publishing.')
const unsigned = process.argv.includes('--unsigned')
const packageInfo = JSON.parse(await readFile(join(root, 'package.json'), 'utf8'))
const version = packageInfo.version
const cargoVersion = /^version\s*=\s*"([^"]+)"/m.exec(await readFile(join(root, 'desktop/Cargo.toml'), 'utf8'))?.[1]
if (cargoVersion !== version) throw new Error(`Version mismatch: package.json=${version}, Cargo.toml=${cargoVersion}`)
const stamp = new Date().toISOString().replace(/[:.]/g, '-')
const output = join(root, 'release', `dioxus-${version}-${stamp}`)
const appDirectory = join(output, 'app')
const runtimePackages = [] // Rust backend; no Node runtime or native Node addons are shipped.
function run(file, args, options = {}) {
  return new Promise((resolve, reject) => {
    const child = spawn(file, args, { cwd: root, stdio: 'inherit', windowsHide: true, ...options })
    child.once('error', reject)
    child.once('exit', code => code === 0 ? resolve() : reject(new Error(`${file} exited ${code}`)))
  })
}
await run(process.execPath, [join(root, 'scripts/build-recorder-helper.mjs'), '--tools-only'])
await run('cargo', ['build', '--release', '--locked', '--manifest-path', 'desktop/Cargo.toml'])
await mkdir(appDirectory, { recursive: true })
await writeFile(join(appDirectory,'version.json'),JSON.stringify({version}))
const turboKeyHelperSource = join(root, 'native/turbo-key/bin/turbo-key-helper.exe')
if (!existsSync(turboKeyHelperSource)) throw new Error('Run npm run native:turbo-key before packaging')
await copyFile(turboKeyHelperSource, join(output, turboKeyHelperAssetName))
await copyFile(join(root, 'desktop/target/release/nogirem-desktop.exe'), join(appDirectory, 'nogirem.exe'))
// Keep source artwork in the repository; runtime uses the optimized JPEG/WebP.
const sourceOnlyAssets = new Set(['public/main3.jpg', 'public/doc_minimize/introduce.png'])
for (const directory of ['assets', 'public']) {
  await cp(join(root, directory), join(appDirectory, directory), {
    recursive: true,
    filter: source => !sourceOnlyAssets.has(relative(root, source).replaceAll('\\', '/')),
  })
}
// Generate only the packaged copy; preserve the full-resolution source artwork.
await run(join(process.env.SystemRoot, 'System32/WindowsPowerShell/v1.0/powershell.exe'), [
  '-NoProfile', '-NonInteractive', '-ExecutionPolicy', 'Bypass',
  '-File', join(root, 'scripts/optimize-package-background.ps1'),
  '-Source', join(root, 'public/main3-optimized.jpg'),
  '-Destination', join(appDirectory, 'public/main3-optimized.jpg'),
])
for (const asset of ['public/main3-optimized.jpg', 'public/doc_minimize/introduce.webp']) {
  if (!existsSync(join(appDirectory, asset))) throw new Error(`Missing runtime image: ${asset}`)
}
await mkdir(join(appDirectory, 'service'))
for (const file of await readdir(join(root, 'service'))) {
  if (!file.endsWith('-preload.js')) continue // WebView renderer bridges only.
  await copyFile(join(root, 'service', file), join(appDirectory, 'service', file))
}
await cp(join(root, 'desktop/target/release/web'), join(appDirectory, 'web'), { recursive: true })
for (const file of ['character-guide.html', 'dxvk-guide.html', 'dxvk-manager.html', 'blackbox-manager.html', 'blackbox-editor.html', 'icon.ico', 'icon-paused.png', 'config.json', 'INTRODUCE.md', 'OPERATION.md', 'TURBO_KEY_TERMS.md', 'VERSION_HISTORY.md', 'VERSION_HISTORY_DETAIL.md', 'NOTICE.md', 'REPORT.json']) {
  await copyFile(join(root, file), join(appDirectory, file))
}
for (const helper of ['input-guard-helper', 'radeon-helper', 'recorder-helper']) {
  await cp(join(root, 'native', helper, 'bin'), join(appDirectory, 'native', helper, 'bin'), { recursive: true })
}
await cp(join(root, "native/recorder-helper/tools-bin"), join(appDirectory, "native/recorder-helper/tools-bin"), { recursive: true })
// Verify the Rust service contract without starting optimizers or loading Node.
const probe = spawn(join(appDirectory, 'nogirem.exe'), ['--native-service', '--contract-test'], {
  cwd: appDirectory, windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
  env: { ...process.env, NOGIREM_ROOT: appDirectory, NOGIREM_PACKAGED: '1', NOGIREM_DESKTOP_EXE: join(appDirectory, 'nogirem.exe') },
})
let probeError = ''
probe.stderr.on('data', data => { probeError += data })
probe.on('error', error => { probeError += error.message })
const connection = createTransport(probe.stdout, probe.stdin, { timeoutMs: 15_000 })
try {
  const contract = await connection.request('contract')
  if (!Array.isArray(contract.channels) || contract.channels.length < 80) throw new Error('Incomplete packaged service contract')
} catch (error) {
  throw new Error(`Packaged service verification failed: ${error.message}\n${probeError}`)
} finally {
  connection.close()
  probe.stdin.end()
  probe.kill()
}

// Reject accidental reintroduction of any Node/Electron backend payload.
async function verifyRustPayload(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (/^(node(?:\.exe)?|node_modules|electron(?:\.exe)?|NODE-LICENSE)$/i.test(entry.name)) throw new Error(`Forbidden runtime payload: ${join(directory, entry.name)}`)
    if (entry.isDirectory()) await verifyRustPayload(join(directory, entry.name))
  }
}
await verifyRustPayload(appDirectory)
for (const file of await readdir(join(appDirectory, 'service'))) {
  if (!file.endsWith('-preload.js')) throw new Error(`Backend JavaScript in package: ${file}`)
}

const portableDirectory = join(output, 'portable')
await cp(appDirectory, portableDirectory, { recursive: true })
await writeFile(join(portableDirectory, 'portable.marker'), 'nogirem-portable-v1\n')
await writeFile(join(portableDirectory, '포터블 사용 안내.txt'), [
  '마비노기 렘 부스터 포터블',
  '',
  '- 배포된 포터블 EXE 하나만 보관하고 실행하면 됩니다.',
  '- 첫 실행에 필요한 내부 파일은 관리자 전용 버전 캐시에 준비되며 다음 실행부터 재사용됩니다.',
  '- 시스템 설정 변경을 위해 실행 시 관리자 권한을 요청합니다.',
  '- Microsoft Edge WebView2 Runtime이 설치되어 있어야 합니다.',
  '- 설정은 설치형과 동일한 AppData 경로를 사용하므로 두 버전에서 공유됩니다.',
  '- 녹화 청크 저장 드라이브는 앱의 블랙박스 설정에서 선택할 수 있습니다.',
  '- 앱 폴더를 옮긴 뒤 실행하면 시작 프로그램 등록 경로가 현재 위치로 갱신됩니다.',
  '',
].join('\r\n'))
// Enumerate only the files shipped by this build. Never recursively delete the
// install directory, which may also contain files created by the user.
const uninstallLines = []
async function enumerateUninstall(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name)
    const destination = relative(appDirectory, path).replaceAll('$', '$$').replaceAll('"', '$\\"')
    if (entry.isDirectory()) {
      await enumerateUninstall(path)
      uninstallLines.push(`RMDir "$INSTDIR\\${destination}"`)
    } else uninstallLines.push(`Delete "$INSTDIR\\${destination}"`)
  }
}
await enumerateUninstall(appDirectory)
const uninstallManifest = join(output, 'uninstall-files.nsh')
await writeFile(uninstallManifest, '\uFEFF' + uninstallLines.join('\r\n'))
const stopScript = join(output, 'stop-installed-app.ps1')
await writeFile(stopScript, '\uFEFF' + (await readFile(join(root, 'scripts/stop-installed-app.ps1'), 'utf8')).replace(/^\uFEFF/, ''))

const bootstrapper = join(output, 'MicrosoftEdgeWebview2Setup.exe')
const response = await fetch('https://go.microsoft.com/fwlink/p/?LinkId=2124703')
if (!response.ok) throw new Error(`WebView2 bootstrapper download failed: ${response.status}`)
await writeFile(bootstrapper, Buffer.from(await response.arrayBuffer()))
const quoted = bootstrapper.replaceAll("'", "''")
await run(join(process.env.SystemRoot, 'System32/WindowsPowerShell/v1.0/powershell.exe'), ['-NoProfile', '-NonInteractive', '-Command', `$ErrorActionPreference='Stop'; $s=Get-AuthenticodeSignature -LiteralPath '${quoted}'; if ($s.Status -ne 'Valid' -or $s.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') { throw 'WebView2 bootstrapper signature verification failed' }`], {
  env: { ...process.env, PSModulePath: join(process.env.SystemRoot, 'System32/WindowsPowerShell/v1.0/Modules') },
})

const candidates = [process.env.NSIS_MAKENSIS, join(process.env.LOCALAPPDATA ?? '', 'electron-builder/Cache/nsis/nsis-3.0.4.1/Bin/makensis.exe'), 'C:/Program Files (x86)/NSIS/makensis.exe'].filter(Boolean)
const nsis = candidates.find(existsSync)
if (!nsis) throw new Error(`NSIS compiler not found. Set NSIS_MAKENSIS. Prepared app: ${appDirectory}`)
const installer = join(output, `nogirem-dioxus-setup-${version}.exe`)
await run(nsis, ['/WX', '/INPUTCHARSET', 'UTF8', `/DAPP_DIRECTORY=${appDirectory}`, `/DWEBVIEW_BOOTSTRAPPER=${bootstrapper}`, `/DUNINSTALL_MANIFEST=${uninstallManifest}`, `/DSTOP_SCRIPT=${stopScript}`, `/DOUTPUT_FILE=${installer}`, `/DAPP_VERSION=${version}`, join(root, 'desktop/installer.nsi')])
const portable = join(output, `nogirem-dioxus-portable-${version}.exe`)
await run(nsis, ['/WX', '/INPUTCHARSET', 'UTF8', `/DAPP_DIRECTORY=${portableDirectory}`, `/DOUTPUT_FILE=${portable}`, `/DAPP_VERSION=${version}`, join(root, 'desktop/portable.nsi')])
const installerBytes = (await stat(installer)).size
const installerLimitBytes = 10_000_000 // Conservative decimal MB limit for file sharing.
if (installerBytes >= installerLimitBytes) throw new Error(`Installer exceeds 10 MB budget: ${installerBytes} bytes`)
const portableBytes = (await stat(portable)).size
await writeFile(join(output, 'build.json'), JSON.stringify({ version, backend: 'Rust', runtime: 'Dioxus Desktop / WebView2', installer, installerBytes, installerLimitBytes, portable, portableBytes, appDirectory, portableDirectory, runtimePackages }, null, 2))
if (!unsigned) await run(process.execPath, [join(root, 'scripts/sign-update.mjs'), installer, portable, version])
console.log(`Installer: ${installer}`)
console.log(`Portable: ${portable}`)
if (unsigned) console.log('Unsigned package: update manifests were not generated')
