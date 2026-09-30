import {execFileSync,spawn} from 'node:child_process'
import {readFile,writeFile,mkdir,copyFile,stat} from 'node:fs/promises'
import {join,dirname,basename} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const legacyRef='3ca3521fe734c94d4485b33e099dc0071c4ba76c' // 마지막 Electron 소스
const bridgeVersion='0.3.19'
const rustVersion=JSON.parse(await readFile(join(root,'package.json'),'utf8')).version
const helper=process.argv[2]
if(!helper) throw new Error('Provide the built Rust nogirem.exe to bundle as the migration helper.')
await stat(helper)
const stamp=new Date().toISOString().replace(/[:.]/g,'-')
const stage=join(root,'release',`electron-bridge-${bridgeVersion}-${stamp}`)
await mkdir(stage,{recursive:true})
const zip=join(stage,'source.zip'),source=join(stage,'source'),output=join(stage,'artifacts')
execFileSync('git',['archive','--format=zip',`--output=${zip}`,legacyRef],{cwd:root,windowsHide:true})
// All paths are supplied as process arguments; no interpolated shell command.
const extract=join(stage,'extract.ps1')
await writeFile(extract,`param([string]$ZipPath,[string]$Target)
$ErrorActionPreference='Stop'
Expand-Archive -LiteralPath $ZipPath -DestinationPath $Target
`)

execFileSync('powershell.exe',['-NoProfile','-NonInteractive','-ExecutionPolicy','Bypass','-File',extract,'-ZipPath',zip,'-Target',source],{windowsHide:true})
const pkg=JSON.parse(await readFile(join(source,'package.json'),'utf8'))
if(!pkg.dependencies?.['electron-updater'] || !pkg.main?.startsWith('electron/')) throw new Error('Pinned legacyRef must be the original Electron source; refusing to construct a bridge from the Rust tree')
pkg.version=bridgeVersion;pkg.build.files.push('migration/nogirem-migrator.exe')
pkg.build.asarUnpack.push('migration/nogirem-migrator.exe')
pkg.build.directories.output=output
pkg.build.artifactName='nogirem-legacy-electron-migration-${version}.${ext}'
await writeFile(join(source,'package.json'),JSON.stringify(pkg,null,2)+'\n')
const lock=JSON.parse(await readFile(join(source,'package-lock.json'),'utf8'));lock.version=bridgeVersion;lock.packages[''].version=bridgeVersion;await writeFile(join(source,'package-lock.json'),JSON.stringify(lock,null,2)+'\n')
await mkdir(join(source,'migration'),{recursive:true});await copyFile(helper,join(source,'migration/nogirem-migrator.exe'))
await copyFile(join(root,'src/migration-attempt.mjs'),join(source,'electron/migration-attempt.mjs'))
const module=`import {app} from 'electron'
import {spawn} from 'node:child_process'
import {readFile} from 'node:fs/promises'
import {join} from 'node:path'
import {randomUUID} from 'node:crypto'
import {createMigrationAttemptGate} from './migration-attempt.mjs'

const targetVersion=${JSON.stringify(rustVersion)}
const statusPath=join(process.env.LOCALAPPDATA ?? app.getPath('userData'),'NogiremUpdater','updates','migration-status.json')
let publish=()=>{}
let monitor=null
let state={phase:'available',percent:0,version:targetVersion,error:null}
let lastStatusKey=''
let lastStatusAt=0
let handedOff=false
const attempts=createMigrationAttemptGate()

function update(next){
 state={...state,...next}
 publish(state)
 return state
}

function failure(message,attempt=attempts.current()){
 if(!attempts.isCurrent(attempt)) return state
 clearInterval(monitor)
 monitor=null
 return update({phase:'error',percent:0,error:{name:'RustMigrationError',message}})
}

async function refresh(attempt){
 if(!attempts.isCurrent(attempt)) return
 try{
  const status=JSON.parse(await readFile(statusPath,'utf8'))
  if(!attempts.isCurrent(attempt)) return
  if(status.attempt!==attempt) return
  const statusKey=\`\${status.updatedAt}:\${status.phase}:\${status.percent}\`
  if(statusKey!==lastStatusKey){
   lastStatusKey=statusKey
   lastStatusAt=Date.now()
  }
  if(status.phase==='error'){
   failure(status.error || 'Rust 버전 전환에 실패했습니다',attempt)
   return
  }
  if(status.phase==='downloading'){
   update({phase:'downloading',percent:Number(status.percent)||0,error:null})
   return
  }
  if(['checking','installing','handoff'].includes(status.phase)){
   if(status.phase==='handoff') handedOff=true
   update({phase:'downloading',percent:status.phase==='checking'?0:99.9,error:null})
  }
 }catch{
  // 관리자 helper가 상태 파일을 만들 때까지 기다린다.
 }
}

export function registerMigration(setState){
 publish=setState
 publish(state)
}

export async function checkMigration(){
 return update({phase:'available',percent:0,version:targetVersion,error:null})
}

export async function requestMigration(){
 if(state.phase==='downloading') return state
 const executable=join(process.resourcesPath,'app.asar.unpacked','migration','nogirem-migrator.exe')
 const attempt=randomUUID()
 attempts.replace(attempt)
 lastStatusKey=''
 lastStatusAt=Date.now()
 handedOff=false
 update({phase:'downloading',percent:0,version:targetVersion,error:null})
 const child=spawn(executable,['--migrate-electron',app.getPath('exe'),String(process.pid),\`--migration-attempt=\${attempt}\`],{windowsHide:true,stdio:['ignore','ignore','pipe']})
 attempts.attach(attempt,child)
 child.stderr.on('data',data=>console.error(\`Rust 전환 helper 오류: \${data.toString().trim()}\`))
 child.once('error',error=>failure(\`Rust 전환 helper를 시작하지 못했습니다: \${error.message}\`,attempt))
 child.once('exit',async code=>{
  await refresh(attempt)
  if(!attempts.isCurrent(attempt)) return
  attempts.clear(attempt)
  if(code && state.phase==='downloading') failure('관리자 권한을 허용하지 않았거나 Rust 전환 helper가 중단되었습니다',attempt)
 })
 clearInterval(monitor)
 monitor=setInterval(()=>{
  if(!attempts.isCurrent(attempt)) return
  if(!handedOff && Date.now()-lastStatusAt>60_000){
   failure('관리자 권한 확인 또는 업데이트 응답이 없어 중단했습니다. 다시 시도해 주세요',attempt)
   attempts.stop(attempt)
   return
  }
  void refresh(attempt)
 },250)
 return state
}

export function scheduleMigration(){
 if(!app.isPackaged || process.argv.includes('--migration-retry-suppressed') || process.argv.some(a=>/^--(?:affinity|memory|network|graphics|nvidia|radeon|nic).*helper/.test(a))) return
 app.whenReady().then(()=>setTimeout(()=>void checkMigration(),3000))
}
`
await writeFile(join(source,'electron/rust-migration.mjs'),module)
const bootstrap=join(source,'electron/bootstrap.mjs')
let text=await readFile(bootstrap,'utf8')
const bootstrapNeedle='await import("./main.mjs")'
if(!text.includes(bootstrapNeedle)) throw new Error('Electron bootstrap migration insertion point is missing')
text=text.replace(bootstrapNeedle,'const migration=await import("./rust-migration.mjs")\n  globalThis.__nogiremRustMigration=migration\n  await import("./main.mjs")\n  migration.scheduleMigration()')
await writeFile(bootstrap,text)
const mainPath=join(source,'electron/main.mjs')
let main=await readFile(mainPath,'utf8')
const stateNeedle='function clearApplicationUpdateCompletionTimer() {'
const checkNeedle='async function checkForApplicationUpdate() {'
const requestNeedle='async function requestApplicationUpdate() {'
for(const needle of [stateNeedle,checkNeedle,requestNeedle]) if(!main.includes(needle)) throw new Error(`Electron update migration insertion point is missing: ${needle}`)
main=main.replace(stateNeedle,'globalThis.__nogiremRustMigration?.registerMigration(setApplicationUpdateState)\n\n'+stateNeedle)
main=main.replace(checkNeedle,checkNeedle+'\n  if (globalThis.__nogiremRustMigration) return globalThis.__nogiremRustMigration.checkMigration()')
main=main.replace(requestNeedle,requestNeedle+'\n  if (globalThis.__nogiremRustMigration) return globalThis.__nogiremRustMigration.requestMigration()')
await writeFile(mainPath,main)
const installerPath=join(source,'build/installer.nsh')
let installerSource=await readFile(installerPath,'utf8')
const globalKill='      ExecShellWait "runas" "$SYSDIR\\taskkill.exe" `/F /IM "${APP_EXECUTABLE_FILENAME}"` SW_HIDE'
if(!installerSource.includes(globalKill)) throw new Error('Electron bridge global process termination insertion point is missing')
installerSource=installerSource.replace(globalKill,()=>[
 `      System::Call 'Kernel32::SetEnvironmentVariable(t "NOGIREM_BRIDGE_TARGET", t "$INSTDIR\\\\\${APP_EXECUTABLE_FILENAME}")'`,
 `      nsExec::ExecToStack '"$SYSDIR\\\\WindowsPowerShell\\\\v1.0\\\\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$target=$$env:NOGIREM_BRIDGE_TARGET; [Diagnostics.Process]::GetProcesses() | ForEach-Object { try { if ($$_.MainModule.FileName -eq $$target) { $$_.Kill() } } catch {} }"'`,
 '      Pop $0',
 '      Pop $1',
].join('\n'))
const processProbe='!insertmacro FIND_PROCESS "${APP_EXECUTABLE_FILENAME}" $R0'
const probeCount=installerSource.split(processProbe).length-1
if(probeCount!==3) throw new Error(`Unexpected Electron bridge process probe count: ${probeCount}`)
const pathProbe=`!macro findBridgeProcess
  System::Call 'Kernel32::SetEnvironmentVariable(t "NOGIREM_BRIDGE_TARGET", t "$INSTDIR\\\\\${APP_EXECUTABLE_FILENAME}")'
  nsExec::ExecToStack '"$SYSDIR\\\\WindowsPowerShell\\\\v1.0\\\\powershell.exe" -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$target=$$env:NOGIREM_BRIDGE_TARGET; $$found=@([Diagnostics.Process]::GetProcesses() | Where-Object { try { $$_.MainModule.FileName -eq $$target } catch { $$false } }); if ($$found.Count -gt 0) { exit 2 }"'
  Pop $R0
  Pop $R2
  \${If} $R0 == 2
    StrCpy $R0 0
  \${ElseIf} $R0 == 0
    StrCpy $R0 1
  \${EndIf}
!macroend

`
installerSource=installerSource.replace('!macro customCheckAppRunning',()=>pathProbe+'!macro customCheckAppRunning')
installerSource=installerSource.split(processProbe).join('!insertmacro findBridgeProcess')
if(/taskkill\.exe[\s\S]{0,80}\/IM/i.test(installerSource)) throw new Error('Electron bridge still contains image-name process termination')
if(installerSource.includes('FIND_PROCESS "${APP_EXECUTABLE_FILENAME}"')) throw new Error('Electron bridge still contains image-name process detection')
await writeFile(installerPath,installerSource)
const appPath=join(source,'web/App.svelte')
let appSource=await readFile(appPath,'utf8')
const availableNeedle='["available", "downloading", "downloaded"].includes(applicationUpdateState.phase)'
const modalNeedle='  || applicationUpdateState.phase === "downloaded"'
const modalProps='    onInstall={installApplicationUpdate}'
const installingNeedle='  let applicationUpdateInstalling = false'
const checkUpdateNeedle=/  function checkApplicationUpdate\(\) \{\r?\n    void window\.nogirem\.checkUpdate\(\)\.catch\(\(\) => \{\}\)\r?\n  \}/
for(const needle of [availableNeedle,modalNeedle,modalProps,installingNeedle]) if(!appSource.includes(needle)) throw new Error(`Electron renderer migration insertion point is missing: ${needle}`)
if(!checkUpdateNeedle.test(appSource)) throw new Error('Electron renderer update retry insertion point is missing')
appSource=appSource.replace(availableNeedle,'["available", "downloading", "downloaded", "error"].includes(applicationUpdateState.phase)')
appSource=appSource.replace(modalNeedle,modalNeedle+'\n  || applicationUpdateState.phase === "error"')
appSource=appSource.replace(installingNeedle,installingNeedle+'\n  let applicationUpdateDismissed = false')
appSource=appSource.replace(checkUpdateNeedle,'  function checkApplicationUpdate() {\n    applicationUpdateDismissed = false\n    void window.nogirem.checkUpdate().catch(() => {})\n  }')
appSource=appSource.replace('{#if !closeModalVisible && (','{#if !closeModalVisible && !applicationUpdateDismissed && (')
appSource=appSource.replace(modalProps,modalProps+'\n    error={applicationUpdateState.phase === "error" ? applicationUpdateState.error?.message : ""}\n    onRetry={checkApplicationUpdate}\n    onDismiss={() => applicationUpdateDismissed = true}')
await writeFile(appPath,appSource)
const modalPath=join(source,'web/UpdatePreviewModal.svelte')
let modalSource=await readFile(modalPath,'utf8')
modalSource=modalSource.replace('  export let onInstall = () => {}','  export let onInstall = () => {}\n  export let error = ""\n  export let onRetry = () => {}\n  export let onDismiss = () => {}')
modalSource=modalSource.replace('{downloaded ? "새 버전 다운로드 완료됨" : "새 버전을 가져오고 있습니다"}','{error ? "업데이트를 완료하지 못했습니다" : downloaded ? "새 버전 다운로드 완료됨" : "새 버전을 가져오고 있습니다"}')
modalSource=modalSource.replace('  <section class="update-preview-panel">','  <section class="update-preview-panel">\n    <button class="update-dismiss" type="button" onclick={onDismiss}>숨기기</button>')
modalSource=modalSource.replace('    {#if !downloaded}', '    {#if error}\n      <p>{error}</p>\n      <button type="button" onclick={onRetry}>다시 시도</button>\n    {:else if !downloaded}')
modalSource=modalSource.replace('  button {', '  p {\n    max-width: 380px;\n    margin: 10px 190px 0 0;\n    font-size: 15px;\n    line-height: 1.45;\n  }\n\n  button {')
modalSource=modalSource.replace('  button:hover {','  button.update-dismiss {\n    top: 10px;\n    right: 14px;\n    bottom: auto;\n    min-width: 68px;\n    height: 32px;\n    padding: 0 10px;\n    font-size: 14px;\n  }\n\n  button:hover {')
if(!modalSource.includes('다시 시도')||!modalSource.includes('export let error')||!appSource.includes('applicationUpdateDismissed')||!modalSource.includes('class="update-dismiss"')) throw new Error('Electron migration retry UI generation failed')
await writeFile(modalPath,modalSource)
await writeFile(join(stage,'bridge-build.json'),JSON.stringify({source,output,version:bridgeVersion,rustVersion,baseCommit:execFileSync('git',['rev-parse',legacyRef],{cwd:root,encoding:'utf8'}).trim(),migrationHelper:helper},null,2))
console.log(`Bridge source prepared: ${source}`)
if(process.argv.includes('--build')){
 const npm=join(dirname(process.execPath),'node_modules/npm/bin/npm-cli.js')
 const run=args=>new Promise((resolve,reject)=>{const c=spawn(process.execPath,[npm,...args],{cwd:source,stdio:'inherit',windowsHide:true,env:{...process.env,CSC_IDENTITY_AUTO_DISCOVERY:'false'}});c.on('error',reject);c.on('exit',code=>code===0?resolve():reject(new Error(`npm ${args[0]} exited ${code}`)))})
 await run(['ci','--no-audit','--no-fund'])
 await run(['run','app:build'])
 await run(['exec','--','electron-builder','--win','nsis','--x64','--publish','never'])
 console.log(`Bridge artifacts: ${output}`)
}
