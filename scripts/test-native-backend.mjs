import assert from 'node:assert/strict'
import {spawn} from 'node:child_process'
import {mkdtemp,writeFile,readFile,unlink,rmdir,access} from 'node:fs/promises'
import {tmpdir} from 'node:os'
import {join,dirname} from 'node:path'
import {fileURLToPath} from 'node:url'
import {setTimeout as delay} from 'node:timers/promises'
const root=dirname(dirname(fileURLToPath(import.meta.url)))
const exe=join(root,'desktop/target/debug/nogirem-desktop.exe')
await access(exe)
const directory=await mkdtemp(join(tmpdir(),'nogirem-native-memory-'))
const statusPath=join(directory,'status.json'),controlPath=join(directory,'control.json')
// A nonexistent game and no purge-on-start make this a read-only lifecycle test.
await writeFile(join(directory,'config.json'),JSON.stringify({gameExecutableName:'nogirem-test-nonexistent.exe',memoryCleaner:{pollIntervalMs:20}}))
const args=['--native-memory-helper',`--status-path=${statusPath}`,`--control-path=${controlPath}`,`--parent-pid=${process.pid}`,'--purge-on-start=false']
const launch=()=>spawn(exe,args,{cwd:directory,env:{...process.env,NOGIREM_ROOT:directory},stdio:['ignore','ignore','pipe'],windowsHide:true})
const worker=launch();let error='';worker.stderr.on('data',v=>error+=v)
const exited=new Promise((resolve,reject)=>{worker.once('error',reject);worker.once('exit',code=>resolve(code))})
async function waitStatus(predicate){for(let i=0;i<150;i++){try{const s=JSON.parse(await readFile(statusPath,'utf8'));if(predicate(s))return s}catch{}await delay(20)}throw new Error(`Worker status timeout: ${error}`)}
try {
 const running=await waitStatus(s=>s.running)
 assert.equal(running.helperPid,worker.pid);assert.equal(running.parentPid,process.pid)
 assert.equal(running.gameActive,false);assert.equal(running.lastCleanup,null);assert.ok(running.current.total>0)
 const duplicate=launch();duplicate.stderr.resume()
 assert.equal(await new Promise((resolve,reject)=>{duplicate.once('error',reject);duplicate.once('exit',resolve)}),1,'duplicate worker must not acquire the same lock')
 await writeFile(controlPath,JSON.stringify({command:'stop',requestedAt:Date.now()}))
 assert.equal(await Promise.race([exited,delay(5000).then(()=>{throw new Error('Worker did not stop')})]),0)
 const stopped=await waitStatus(s=>s.running===false)
 assert.equal(stopped.gameActive,false);assert.equal(stopped.lastCleanup,null)
 await assert.rejects(access(`${statusPath}.lock`))
 console.log('Rust memory worker: real native process, read-only sampling, duplicate protection, graceful stop and lock cleanup passed.')
} finally {
 if(worker.exitCode===null) {await writeFile(controlPath,JSON.stringify({command:'stop'}));await Promise.race([exited,delay(2000)]);if(worker.exitCode===null)worker.kill()}
 // Only this test's explicit temporary files are removed.
 for(const name of ['status.json','control.json','status.json.lock','config.json'])await unlink(join(directory,name)).catch(()=>{})
 await rmdir(directory)
}

const affinityDirectory=await mkdtemp(join(tmpdir(),'nogirem-native-affinity-'))
const affinityStatus=join(affinityDirectory,'status.json'),affinityControl=join(affinityDirectory,'control.json')
await writeFile(join(affinityDirectory,'config.json'),JSON.stringify({gameExecutableName:'nogirem-test-nonexistent.exe',pollIntervalMs:50,excludeNames:[],excludeNamePatterns:[]}))
const affinityWorker=spawn(exe,['--native-affinity-helper',`--status-path=${affinityStatus}`,`--control-path=${affinityControl}`,`--affinity-state-path=${join(affinityDirectory,'runtime-state.json')}`,`--applied-marker-path=${join(affinityDirectory,'applied-marker.json')}`,`--game-path-state-path=${join(affinityDirectory,'game-path.json')}`,'--include-nic=false','--nic-managed=false'],{cwd:affinityDirectory,env:{...process.env,NOGIREM_ROOT:affinityDirectory},stdio:['ignore','ignore','pipe'],windowsHide:true})
let affinityError='';affinityWorker.stderr.on('data',v=>affinityError+=v)
const affinityExited=new Promise((resolve,reject)=>{affinityWorker.once('error',reject);affinityWorker.once('exit',resolve)})
try {
 let status
 for(let i=0;i<150;i++){try{status=JSON.parse(await readFile(affinityStatus,'utf8'));if(status.running)break}catch{}await delay(30)}
 assert.equal(status?.running,true,affinityError)
 assert.equal(status.gameActive,false);assert.equal(status.nicManaged,false);assert.equal(status.helperPid,affinityWorker.pid)
 assert.equal(status.renderer.mode,'not-running');assert.ok(status.cpuTopology.backgroundCpuRange)
 await writeFile(affinityControl,JSON.stringify({command:'stop',requestedAt:Date.now()}))
 assert.equal(await Promise.race([affinityExited,delay(5000).then(()=>{throw new Error('Affinity worker did not stop')})]),0,affinityError)
 status=JSON.parse(await readFile(affinityStatus,'utf8'))
 assert.equal(status.running,false);assert.equal(status.exitAction,'keep');assert.equal(status.appliedMarkerRecorded,false)
 await assert.rejects(access(`${affinityStatus}.lock`))
 console.log('Rust affinity worker: game absent, no user process affinity changes, native status, graceful keep/stop passed.')
} finally {
 if(affinityWorker.exitCode===null){await writeFile(affinityControl,JSON.stringify({command:'stop'}));await Promise.race([affinityExited,delay(2000)]);if(affinityWorker.exitCode===null)affinityWorker.kill()}
 for(const name of ['config.json','status.json','control.json','status.json.lock','runtime-state.json','game-path.json','applied-marker.json'])await unlink(join(affinityDirectory,name)).catch(()=>{})
 await rmdir(affinityDirectory)
}
const {execFile}=await import('node:child_process')
const {promisify}=await import('node:util')
const {stdout}=await promisify(execFile)(exe,['--native-operation','auto-tuning',JSON.stringify({applyChanges:false})],{windowsHide:true,timeout:40000})
const network=JSON.parse(stdout)
assert.equal(network.ok,true);assert.equal(network.data.applied,false);assert.equal(typeof network.data.current.effective,'string')
console.log('Rust native operation: TCP auto-tuning read-only query passed.')
