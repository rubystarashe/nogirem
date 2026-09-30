import {existsSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {copyFile,mkdir,readFile,readdir,stat,writeFile} from 'node:fs/promises'
import {join} from 'node:path'

export function releaseAssetNames(version){
  return {
    installer:`nogirem-dioxus-setup-${version}.exe`,
    portable:`nogirem-dioxus-portable-${version}.exe`,
    publicInstaller:`nogirem-setup-${version}.exe`,
    publicPortable:`nogirem-portable-${version}.exe`,
    legacy:'nogirem-legacy-electron-migration-0.3.19.exe',
    oldLegacy:'nogirem-setup-0.3.19.exe',
  }
}

export async function assembleReleaseAssets({rust,bridge,destination,version,turboKeyHelperAssetName}){
  const names=releaseAssetNames(version)
  const legacySourceName=existsSync(join(bridge,names.legacy))?names.legacy:names.oldLegacy
  const legacy=await readFile(join(bridge,legacySourceName))
  const sourceYml=await readFile(join(bridge,'latest.yml'),'utf8')
  if(!/^version:\s*0\.3\.19\s*$/m.test(sourceYml)||!sourceYml.includes(legacySourceName)) throw new Error('latest.yml MUST point to the Electron 0.3.19 migration bridge')
  const sha512=createHash('sha512').update(legacy).digest('base64')
  const sha512Entries=[...sourceYml.matchAll(/sha512:\s*(\S+)/g)]
  if(sha512Entries.length<2) throw new Error('Electron latest.yml checksums are missing')
  for(const match of sha512Entries) if(match[1]!==sha512) throw new Error('Electron latest.yml checksum mismatch')
  const blockmapSource=join(bridge,legacySourceName+'.blockmap')
  if(!existsSync(blockmapSource)) throw new Error('Electron migration blockmap is required')
  const blockmapSize=(await stat(blockmapSource)).size
  if(blockmapSize===0) throw new Error('Electron migration blockmap is empty')
  const declaredBlockmapSize=/^blockMapSize:\s*(\d+)\s*$/m.exec(sourceYml)?.[1]
  if(declaredBlockmapSize&&Number(declaredBlockmapSize)!==blockmapSize) throw new Error('Electron migration blockmap size mismatch')
  const yml=sourceYml.replaceAll(legacySourceName,names.legacy)

  await mkdir(destination,{recursive:true})
  for(const file of [names.installer,'update.json',names.portable,'portable-update.json',turboKeyHelperAssetName]) await copyFile(join(rust,file),join(destination,file))
  await copyFile(join(rust,names.installer),join(destination,names.publicInstaller))
  await copyFile(join(rust,names.portable),join(destination,names.publicPortable))
  await copyFile(join(bridge,legacySourceName),join(destination,names.legacy))
  await copyFile(blockmapSource,join(destination,names.legacy+'.blockmap'))
  await writeFile(join(destination,'latest.yml'),yml)
  return {files:await readdir(destination),names}
}
