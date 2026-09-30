import {turboKeyHelperAssetName} from '../src/turbo-key-installer.mjs'
import {readFile,writeFile} from 'node:fs/promises'
import {createHash,createPublicKey,verify} from 'node:crypto'
import {join,dirname} from 'node:path'
import {fileURLToPath} from 'node:url'
import {assembleReleaseAssets,releaseAssetNames} from './release-assets.mjs'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const [rust,bridge]=process.argv.slice(2)
if(!rust || !bridge) throw new Error('Usage: node scripts/prepare-release.mjs <Rust build directory> <Electron bridge artifacts directory>')
const cfg=JSON.parse(await readFile(join(root,'build-config/update.json'),'utf8'))
const updateBytes=await readFile(join(rust,'update.json'))
const update=JSON.parse(updateBytes),payload=JSON.parse(update.payload)
const names=releaseAssetNames(payload.version)
const publicKey=createPublicKey({key:Buffer.concat([Buffer.from('302a300506032b6570032100','hex'),Buffer.from(cfg.publicKey,'base64')]),type:'spki',format:'der'})
if(!verify(null,Buffer.from(update.payload),publicKey,Buffer.from(update.signature,'base64'))) throw new Error('Invalid update signature')
const installer=await readFile(join(rust,names.installer))
if(installer.length!==payload.size || createHash('sha256').update(installer).digest('hex')!==payload.sha256) throw new Error('Rust artifact mismatch')
const portableUpdate=JSON.parse(await readFile(join(rust,'portable-update.json'),'utf8'))
const portablePayload=JSON.parse(portableUpdate.payload)
if(!verify(null,Buffer.from(portableUpdate.payload),publicKey,Buffer.from(portableUpdate.signature,'base64'))) throw new Error('Invalid portable update signature')
const portable=await readFile(join(rust,names.portable))
if(portablePayload.version!==payload.version || portable.length!==portablePayload.size || createHash('sha256').update(portable).digest('hex')!==portablePayload.sha256) throw new Error('Portable artifact mismatch')
const destination=join(root,'release',`ready-v${payload.version}-${new Date().toISOString().replace(/[:.]/g,'-')}`)
const {files}=await assembleReleaseAssets({rust,bridge,destination,version:payload.version,turboKeyHelperAssetName})
await writeFile(join(destination,'release-plan.json'),JSON.stringify({repository:cfg.repository,tag:`v${payload.version}`,rustVersion:payload.version,electronBridgeVersion:'0.3.19',files,verified:true,published:false},null,2))
await writeFile(join(destination,'README.txt'),`Upload ALL listed artifacts to GitHub ${cfg.repository} release v${payload.version}.\nKeep latest.yml at version 0.3.19 and include its installer in EVERY future Rust release.\nRust reads signed update.json; Electron reads latest.yml.\nDo not publish until Windows migration/rollback acceptance tests are complete.\nThis command does not publish, commit or push.\n`)
console.log(destination)
