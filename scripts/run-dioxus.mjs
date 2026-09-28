import { existsSync } from 'node:fs'
import { spawn } from 'node:child_process'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
const root = dirname(dirname(fileURLToPath(import.meta.url)))
if (!existsSync(join(root,'native/recorder-helper/tools-bin/recorder-helper.exe'))) {
  await new Promise((resolve,reject)=>{const helper=spawn(process.execPath,[join(root,'scripts/build-recorder-helper.mjs'),'--tools-only'],{cwd:root,stdio:'inherit',windowsHide:true});helper.on('error',reject);helper.on('exit',code=>code===0?resolve():reject(new Error('Recorder tools build failed')));});
}
const args = ['run', '--locked', '--manifest-path', join(root, 'desktop/Cargo.toml'), '--', ...process.argv.slice(2)]
const child = spawn('cargo', args, { cwd: root, stdio: 'inherit', windowsHide: true })
child.on('error', error => { console.error(error.message); process.exitCode = 1 })
child.on('exit', code => { process.exitCode = code ?? 1 })
