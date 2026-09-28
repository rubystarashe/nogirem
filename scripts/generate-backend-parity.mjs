import { writeFile } from 'node:fs/promises'
import { resolve, dirname } from 'node:path'
import { fileURLToPath, pathToFileURL } from 'node:url'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const url=path=>pathToFileURL(resolve(root,path))
const settings=await import(url('src/blackbox-settings.mjs'))
const turbo=await import(url('src/turbo-key-settings.mjs'))
const affinity=await import(url('src/affinity.mjs'))
const plain=v=>JSON.parse(JSON.stringify(v,(_,v)=>typeof v==='bigint'?Number(v):v))
const inputs=[null,{}, {enabled:true}, {featureEnabled:true}, {maxDurationSeconds:null}]
for(const value of [-5,0,59,60,60.5,3600,604800,604801,'60','bad',null,true,false])inputs.push({maxDurationSeconds:value})
for(const codec of ['h264','hevc','wrong'])for(const quality of ['auto','1080p','1440p','original','wrong'])for(const fps of [30,60,90])inputs.push({codec,quality,fps})
for(const ringStorageDrive of ['C:','d:','D:\\','../','한:',''])inputs.push({ringStorageDrive})
for(const ringStoragePath of ['d:\\legacy\\ring','e:/ring','C:relative',''])inputs.push({ringStoragePath})
for(const shortcut of ['',' ','ctrl+shift+F10','Ctrl+PauseBreak','Pause','F1','F01','Ctrl+F24','Ctrl+F25','Ctrl+A+B','Meta+Meta+F12','PrintScreen','CTRL+numadd','shift+ctrl+alt+F1'])inputs.push({shortcut})
const blackbox=inputs.map(input=>({input,normalized:settings.normalizeBlackboxSetting(input),bitrate:settings.bitrateForBlackboxSetting(input),low:settings.resolveBlackboxQuality(input,{logicalCpuCount:8,totalMemoryBytes:8*1024**3}),high:settings.resolveBlackboxQuality(input,{logicalCpuCount:16,totalMemoryBytes:32*1024**3})}))
const topology=[]
for(const performance of [2,4,6,8,12])for(const efficient of [0,4,8])for(const requested of [null,-3,1,2,4,8,99]){
 const cpus=[]
 for(let i=0;i<performance*2;i++)cpus.push({group:0,logicalProcessorIndex:i,coreIndex:Math.floor(i/2),efficiencyClass:efficient?1:0})
 for(let i=0;i<efficient;i++)cpus.push({group:0,logicalProcessorIndex:performance*2+i,coreIndex:performance+i,efficiencyClass:0})
 topology.push({cpuSets:cpus,count:cpus.length,requested,expected:plain(affinity.buildCpuTopologyMasks(cpus,cpus.length,requested))})
}
const keys=[[],[65,65,1,300,-1,96,123,124,186,222],Array.from({length:256},(_,i)=>i),['65',65.5,null,true]]
await writeFile(resolve(root,'desktop/backend/tests/legacy-parity.json'),JSON.stringify({blackbox,topology,keys:keys.map(input=>({input,expected:turbo.normalizeTurboKeyCodes(input)}))},null,2))
console.log(`Recorded ${blackbox.length+topology.length+keys.length} reference cases`)
