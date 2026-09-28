import {execFileSync,spawn} from 'node:child_process'
import {readFile,writeFile,mkdir,copyFile,stat} from 'node:fs/promises'
import {join,dirname,basename} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const legacyRef='3ca3521fe734c94d4485b33e099dc0071c4ba76c' // Last Electron source.
const helper=process.argv[2]
if(!helper) throw new Error('Provide the built Rust nogirem.exe to bundle as the migration helper.')
await stat(helper)
const stamp=new Date().toISOString().replace(/[:.]/g,'-')
const stage=join(root,'release',`electron-bridge-0.3.17-${stamp}`)
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
pkg.version='0.3.17';pkg.build.files.push('migration/nogirem-migrator.exe')
pkg.build.asarUnpack.push('migration/nogirem-migrator.exe')
pkg.build.directories.output=output
await writeFile(join(source,'package.json'),JSON.stringify(pkg,null,2)+'\n')
const lock=JSON.parse(await readFile(join(source,'package-lock.json'),'utf8'));lock.version='0.3.17';lock.packages[''].version='0.3.17';await writeFile(join(source,'package-lock.json'),JSON.stringify(lock,null,2)+'\n')
await mkdir(join(source,'migration'),{recursive:true});await copyFile(helper,join(source,'migration/nogirem-migrator.exe'))
const module=`import {app} from 'electron'\nimport {spawn} from 'node:child_process'\nimport {join} from 'node:path'\nexport function scheduleMigration(){\n if(!app.isPackaged || process.argv.includes('--migration-retry-suppressed') || process.argv.some(a=>/^--(?:affinity|memory|network|graphics|nvidia|radeon|nic).*helper/.test(a))) return\n app.whenReady().then(()=>setTimeout(()=>{\n  const executable=join(process.resourcesPath,'app.asar.unpacked','migration','nogirem-migrator.exe')\n  const child=spawn(executable,['--migrate-electron',app.getPath('exe'),String(process.pid)],{detached:true,windowsHide:true,stdio:'ignore'})\n  child.on('error',error=>console.error('Rust 전환 시작 실패',error))\n  child.unref()\n },8000))\n}\n`
await writeFile(join(source,'electron/rust-migration.mjs'),module)
const bootstrap=join(source,'electron/bootstrap.mjs')
let text=await readFile(bootstrap,'utf8');text=text.replace('await import("./main.mjs")','await import("./main.mjs")\n  const {scheduleMigration}=await import("./rust-migration.mjs")\n  scheduleMigration()');await writeFile(bootstrap,text)
await writeFile(join(stage,'bridge-build.json'),JSON.stringify({source,output,version:'0.3.17',baseCommit:execFileSync('git',['rev-parse',legacyRef],{cwd:root,encoding:'utf8'}).trim(),migrationHelper:helper},null,2))
console.log(`Bridge source prepared: ${source}`)
if(process.argv.includes('--build')){
 const npm=join(dirname(process.execPath),'node_modules/npm/bin/npm-cli.js')
 const run=args=>new Promise((resolve,reject)=>{const c=spawn(process.execPath,[npm,...args],{cwd:source,stdio:'inherit',windowsHide:true,env:{...process.env,CSC_IDENTITY_AUTO_DISCOVERY:'false'}});c.on('error',reject);c.on('exit',code=>code===0?resolve():reject(new Error(`npm ${args[0]} exited ${code}`)))})
 await run(['ci','--no-audit','--no-fund'])
 await run(['run','app:build'])
 await run(['exec','--','electron-builder','--win','nsis','--x64','--publish','never'])
 console.log(`Bridge artifacts: ${output}`)
}
