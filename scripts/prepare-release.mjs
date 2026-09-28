import {turboKeyHelperAssetName} from '../src/turbo-key-installer.mjs'
import {execFileSync} from 'node:child_process'
import {readFile,writeFile,mkdir,copyFile,readdir,stat} from 'node:fs/promises'
import {createHash,createPublicKey,verify} from 'node:crypto'
import {join,dirname} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const [rust,bridge]=process.argv.slice(2)
if(!rust || !bridge) throw new Error('Usage: node scripts/prepare-release.mjs <Rust build directory> <Electron bridge artifacts directory>')
const cfg=JSON.parse(await readFile(join(root,'build-config/update.json'),'utf8'))
const updateBytes=await readFile(join(rust,'update.json'))
const update=JSON.parse(updateBytes),payload=JSON.parse(update.payload)
const publicKey=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),Buffer.from(cfg.publicKey,'base64')]),type:'spki',format:'der'})
if(!verify(null,Buffer.from(update.payload),publicKey,Buffer.from(update.signature,'base64'))) throw new Error('Invalid update signature')
const name=`nogirem-dioxus-setup-${payload.version}.exe`,installer=await readFile(join(rust,name))
if(installer.length!==payload.size || createHash('sha256').update(installer).digest('hex')!==payload.sha256) throw new Error('Rust artifact mismatch')
const portableUpdate=JSON.parse(await readFile(join(rust,'portable-update.json'),'utf8'))
const portablePayload=JSON.parse(portableUpdate.payload)
if(!verify(null,Buffer.from(portableUpdate.payload),publicKey,Buffer.from(portableUpdate.signature,'base64'))) throw new Error('Invalid portable update signature')
const portableName=`nogirem-dioxus-portable-${payload.version}.zip`,portable=await readFile(join(rust,portableName))
if(portablePayload.version!==payload.version || portable.length!==portablePayload.size || createHash('sha256').update(portable).digest('hex')!==portablePayload.sha256) throw new Error('Portable artifact mismatch')
execFileSync(process.execPath,[join(root,'scripts/verify-portable-package.mjs'),join(rust,portableName),payload.version,'--require-signature'],{stdio:'inherit'})
const legacyName='nogirem-setup-0.3.17.exe',legacy=await readFile(join(bridge,legacyName)),yml=await readFile(join(bridge,'latest.yml'),'utf8')
if(!/^version:\s*0\.3\.17\s*$/m.test(yml) || !yml.includes(legacyName)) throw new Error('latest.yml MUST remain the Electron 0.3.17 migration bridge')
const sha512=createHash('sha512').update(legacy).digest('base64')
for(const match of yml.matchAll(/sha512:\s*(\S+)/g)) if(match[1]!==sha512) throw new Error('Electron latest.yml checksum mismatch')
const destination=join(root,'release',`ready-v${payload.version}-${new Date().toISOString().replace(/[:.]/g,'-')}`)
await mkdir(destination,{recursive:true})
for(const file of [name,'update.json',portableName,'portable-update.json',turboKeyHelperAssetName]) await copyFile(join(rust,file),join(destination,file))
for(const file of [legacyName,'latest.yml',...(await readdir(bridge)).filter(n=>n===legacyName+'.blockmap')]) await copyFile(join(bridge,file),join(destination,file))
const files=await readdir(destination)
await writeFile(join(destination,'release-plan.json'),JSON.stringify({repository:cfg.repository,tag:`v${payload.version}`,rustVersion:payload.version,electronBridgeVersion:'0.3.17',files,verified:true,published:false},null,2))
await writeFile(join(destination,'README.txt'),`Upload ALL listed artifacts to GitHub ${cfg.repository} release v${payload.version}.\nKeep latest.yml at version 0.3.17 and include its installer in EVERY future Rust release.\nRust reads signed update.json; Electron reads latest.yml.\nDo not publish until Windows migration/rollback acceptance tests are complete.\nThis command does not publish, commit or push.\n`)
console.log(destination)
