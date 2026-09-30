import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdtemp,mkdir,readFile,rm,writeFile} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join} from 'node:path'
import test from 'node:test'
import {assembleReleaseAssets} from '../scripts/release-assets.mjs'

async function fixture(t,{blockmap=true}={}){
  const root=await mkdtemp(join(tmpdir(),'nogirem-release-assets-'))
  t.after(()=>rm(root,{recursive:true,force:true}))
  const rust=join(root,'rust'),bridge=join(root,'bridge'),destination=join(root,'ready')
  await mkdir(rust,{recursive:true})
  await mkdir(bridge,{recursive:true})
  const installer=Buffer.from('rust-installer')
  const portable=Buffer.from('rust-portable')
  const legacy=Buffer.from('electron-migration')
  const legacyName='nogirem-setup-0.3.19.exe'
  const blockmapBytes=Buffer.from('electron-blockmap')
  for(const [name,bytes] of [
    ['nogirem-dioxus-setup-0.4.2.exe',installer],
    ['nogirem-dioxus-portable-0.4.2.exe',portable],
    ['update.json',Buffer.from('update')],
    ['portable-update.json',Buffer.from('portable-update')],
    ['turbo.exe',Buffer.from('turbo')],
  ]) await writeFile(join(rust,name),bytes)
  await writeFile(join(bridge,legacyName),legacy)
  if(blockmap) await writeFile(join(bridge,legacyName+'.blockmap'),blockmapBytes)
  const sha512=createHash('sha512').update(legacy).digest('base64')
  await writeFile(join(bridge,'latest.yml'),`version: 0.3.19\nfiles:\n  - url: ${legacyName}\n    sha512: ${sha512}\n    size: ${legacy.length}\npath: ${legacyName}\nsha512: ${sha512}\nblockMapSize: ${blockmapBytes.length}\n`)
  return {rust,bridge,destination,installer,portable,legacy,blockmapBytes}
}

test('릴리스 조립은 권장 alias와 자동 업데이트 호환 자산을 함께 보존한다',async t=>{
  const input=await fixture(t)
  const {files}=await assembleReleaseAssets({...input,version:'0.4.2',turboKeyHelperAssetName:'turbo.exe'})
  assert.deepEqual(files.sort(),[
    'latest.yml',
    'nogirem-dioxus-portable-0.4.2.exe',
    'nogirem-dioxus-setup-0.4.2.exe',
    'nogirem-legacy-electron-migration-0.3.19.exe',
    'nogirem-legacy-electron-migration-0.3.19.exe.blockmap',
    'nogirem-portable-0.4.2.exe',
    'nogirem-setup-0.4.2.exe',
    'portable-update.json',
    'turbo.exe',
    'update.json',
  ])
  assert.deepEqual(await readFile(join(input.destination,'nogirem-setup-0.4.2.exe')),input.installer)
  assert.deepEqual(await readFile(join(input.destination,'nogirem-dioxus-setup-0.4.2.exe')),input.installer)
  assert.deepEqual(await readFile(join(input.destination,'nogirem-portable-0.4.2.exe')),input.portable)
  assert.deepEqual(await readFile(join(input.destination,'nogirem-dioxus-portable-0.4.2.exe')),input.portable)
  assert.deepEqual(await readFile(join(input.destination,'nogirem-legacy-electron-migration-0.3.19.exe')),input.legacy)
  assert.deepEqual(await readFile(join(input.destination,'nogirem-legacy-electron-migration-0.3.19.exe.blockmap')),input.blockmapBytes)
  const yml=await readFile(join(input.destination,'latest.yml'),'utf8')
  assert.match(yml,/nogirem-legacy-electron-migration-0\.3\.19\.exe/)
  assert.doesNotMatch(yml,/url: nogirem-setup-0\.3\.19\.exe/)
})

test('릴리스 조립은 Electron blockmap 누락을 거부한다',async t=>{
  const input=await fixture(t,{blockmap:false})
  await assert.rejects(()=>assembleReleaseAssets({...input,version:'0.4.2',turboKeyHelperAssetName:'turbo.exe'}),/blockmap is required/)
})

test('릴리스 조립은 Electron sha512 누락을 거부한다',async t=>{
  const input=await fixture(t)
  const ymlPath=join(input.bridge,'latest.yml')
  const yml=(await readFile(ymlPath,'utf8')).replace(/^.*sha512:.*$\n?/gm,'')
  await writeFile(ymlPath,yml)
  await assert.rejects(()=>assembleReleaseAssets({...input,version:'0.4.2',turboKeyHelperAssetName:'turbo.exe'}),/checksums are missing/)
})
