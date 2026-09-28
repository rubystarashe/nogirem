import {generateKeyPairSync,createPrivateKey,createPublicKey} from 'node:crypto'
import {mkdir,readFile,writeFile} from 'node:fs/promises'
import {existsSync} from 'node:fs'
import {join,dirname} from 'node:path'
import {fileURLToPath} from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const keyPath=process.env.NOGIREM_UPDATE_PRIVATE_KEY || join(process.env.LOCALAPPDATA,'NogiremReleaseKeys','update-ed25519.pem')
await mkdir(dirname(keyPath),{recursive:true})
if(!existsSync(keyPath)) {
 const {privateKey}=generateKeyPairSync('ed25519')
 await writeFile(keyPath,privateKey.export({type:'pkcs8',format:'pem'}),{flag:'wx',mode:0o600})
}
const privateKey=createPrivateKey(await readFile(keyPath))
const publicKey=createPublicKey(privateKey).export({type:'spki',format:'der'}).subarray(-32).toString('base64')
const path=join(root,'build-config','update.json')
if(existsSync(path)) {
 const old=JSON.parse(await readFile(path,'utf8'))
 if(old.publicKey!==publicKey) throw new Error('Existing updater public key differs; key rotation requires an explicit migration.')
} else {
 await mkdir(dirname(path),{recursive:true})
 await writeFile(path,JSON.stringify({repository:'rubystarashe/nogirem',publicKey},null,2)+'\n')
}
console.log('Update signing configuration ready. Private key remains outside repository: '+keyPath)
