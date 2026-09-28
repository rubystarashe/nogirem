import {createPrivateKey,createPublicKey,sign,verify,createHash} from 'node:crypto'
import {readFile,writeFile,stat} from 'node:fs/promises'
import {join,dirname,basename} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const [installer,portable,version]=process.argv.slice(2)
if(!installer || !portable || !/^\d+\.\d+\.\d+$/.test(version)) throw new Error('Usage: node scripts/sign-update.mjs <installer> <portable> <version>')
if(basename(installer)!==`nogirem-dioxus-setup-${version}.exe`) throw new Error('Unexpected installer filename')
if(basename(portable)!==`nogirem-dioxus-portable-${version}.exe`) throw new Error('Unexpected portable filename')
const cfg=JSON.parse(await readFile(join(root,'build-config/update.json'),'utf8'))
const key=createPrivateKey(await readFile(process.env.NOGIREM_UPDATE_PRIVATE_KEY || join(process.env.LOCALAPPDATA,'NogiremReleaseKeys','update-ed25519.pem')))
const pub=createPublicKey(key).export({type:'spki',format:'der'}).subarray(-32).toString('base64')
if(pub!==cfg.publicKey) throw new Error('Signing key does not match the key embedded in the app')
async function createEnvelope(file,name) {
  const bytes=await readFile(file)
  const payload=JSON.stringify({schemaVersion:1,version,url:`https://github.com/${cfg.repository}/releases/download/v${version}/${basename(file)}`,size:(await stat(file)).size,sha256:createHash('sha256').update(bytes).digest('hex'),notes:`렘 부스터 ${version} 업데이트`})
  const signature=sign(null,Buffer.from(payload),key)
  if(!verify(null,Buffer.from(payload),createPublicKey(key),signature)) throw new Error('Signature self-verification failed')
  await writeFile(join(dirname(installer),name),JSON.stringify({payload,signature:signature.toString('base64')},null,2)+'\n')
}
await createEnvelope(installer,'update.json')
await createEnvelope(portable,'portable-update.json')
console.log('Signed installer and portable update manifests generated and verified.')
