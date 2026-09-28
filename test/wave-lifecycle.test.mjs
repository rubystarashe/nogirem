import assert from 'node:assert/strict'
import test from 'node:test'
import vm from 'node:vm'
import {readFileSync} from 'node:fs'
const source=readFileSync(new URL('../desktop/src/wave.js',import.meta.url),'utf8')
function fixture() {
  let now=0,id=0; const timers=new Map(),images=[]
  const context=new Proxy({}, {get:(_,key)=>key==='getImageData'?()=>({data:new Uint8ClampedArray(4)}):()=>{}})
  const canvas=()=>({getContext:()=>context})
  const window={setTimeout:(fn,delay)=>{timers.set(++id,{fn,at:now+delay});return id},clearTimeout:i=>timers.delete(i),addEventListener(){},removeEventListener(){}}
  const sandbox={window,performance:{now:()=>now},Math,console,Uint8ClampedArray,
    document:{createElement:canvas,querySelector:()=>null,documentElement:{classList:{toggle(){},remove(){}}}},
    Audio:class {load(){} pause(){}}, Image:class {width=640;height=290;constructor(){images.push(this)}},
    requestAnimationFrame:()=>++id,cancelAnimationFrame(){}}
  vm.runInNewContext(source,sandbox)
  const wave=window.createNogiremWave(canvas(),()=>{})
  images.forEach(image=>image.onload())
  function advance(ms){const until=now+ms;while(true){const next=[...timers].sort((a,b)=>a[1].at-b[1].at)[0];if(!next||next[1].at>until)break;now=next[1].at;timers.delete(next[0]);next[1].fn()}now=until}
  return {wave,advance}
}
test('startup completion waits 2 seconds before ambient waves',()=>{
  const {wave,advance}=fixture();wave.finishStartup();advance(450)
  assert.equal(wave.inspect().ambientCircles,0)
  advance(1999);assert.equal(wave.inspect().ambientCircles,0)
  advance(1);assert.equal(wave.inspect().ambientCircles,10)
  advance(6200);assert.equal(wave.inspect().ambientPending,true)
})
test('last startup wave cue enforces the full 2 second ambient delay',()=>{
  const {wave,advance}=fixture();wave.setStartupMuted(true);wave.allowStartup()
  advance(1500);wave.finishStartup();advance(4449)
  assert.equal(wave.inspect().ambientCircles,0)
  advance(1);assert.equal(wave.inspect().ambientCircles,10)
})
test('skipping startup schedules waves and blur clears them without errors',()=>{
  const {wave,advance}=fixture();wave.skipStartup();advance(1200)
  assert.equal(wave.inspect().circles,10)
  wave.setPageVisible(false);assert.equal(wave.inspect().circles,0)
  assert.doesNotThrow(()=>wave.setLogoVisible(false))
  assert.equal(wave.inspect().ambientPending,false)
  wave.setPageVisible(true);advance(300);assert.equal(wave.inspect().circles,10)
})
