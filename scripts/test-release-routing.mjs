import {createRequire} from 'node:module'
import {readFile} from 'node:fs/promises'
import {resolve,join} from 'node:path'
import assert from 'node:assert/strict'
const stage=resolve(process.argv[2] || '')
const require=createRequire(join(stage,'source/package.json'))
const {GitHubProvider}=require('electron-updater/out/providers/GitHubProvider.js')
const semver=require('semver')
const yml=await readFile(join(stage,'artifacts/latest.yml'),'utf8')
for(const tag of ['v0.4.0','v0.4.1']){
 const requests=[]
 const executor={request:async options=>{
  const path=options.path;requests.push(path)
  if(path==='/rubystarashe/nogirem/releases.atom') return `<feed><entry><link href="https://github.com/rubystarashe/nogirem/releases/tag/${tag}"/><title>Rust ${tag}</title><content>Update</content></entry></feed>`
  if(path==='/rubystarashe/nogirem/releases/latest') return JSON.stringify({tag_name:tag})
  if(path===`/rubystarashe/nogirem/releases/download/${tag}/latest.yml`) return yml
  throw new Error(`Unexpected request ${path}`)
 }}
 const provider=new GitHubProvider({owner:'rubystarashe',repo:'nogirem'},{currentVersion:new semver.SemVer('0.3.16'),allowPrerelease:false,fullChangelog:false},{platform:'win32',executor})
 const info=await provider.getLatestVersion()
 assert.equal(info.version,'0.3.18')
 assert(semver.gt(info.version,'0.3.17'))
 assert(!semver.gt(info.version,'0.3.18'))
 const files=provider.resolveFiles(info)
 assert.equal(files[0].url.href,`https://github.com/rubystarashe/nogirem/releases/download/${tag}/nogirem-setup-0.3.18.exe`)
 console.log(JSON.stringify({tag,delivers:info.version,url:files[0].url.href,passed:true}))
}
