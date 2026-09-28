import test from 'node:test'
import assert from 'node:assert/strict'
import { spawn } from 'node:child_process'
import { readFile, readdir } from 'node:fs/promises'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { createTransport } from './transport.mjs'

const directory = dirname(fileURLToPath(import.meta.url))
test('native service registers every renderer IPC route without loading Electron or applying settings', async t => {
  const child = spawn(process.execPath, [join(directory, 'main.mjs'), '--contract-test'], {
    cwd: join(directory, '..'), windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'],
    env: { ...process.env, NOGIREM_DESKTOP_EXE: process.execPath },
  })
  const client = createTransport(child.stdout, child.stdin, { timeoutMs: 10_000 })
  let stderr = ''
  child.stderr.on('data', data => { stderr += data })
  t.after(() => { client.close(); child.stdin.end(); child.kill() })
  let contract
  try { contract = await client.request('contract') }
  catch (error) { throw new Error(`${error.message}\n${stderr}`) }
  const registered = new Set(contract.channels)
  const events = new Set(contract.events)
  for (const file of await readdir(directory)) {
    if (!file.endsWith('preload.js')) continue
    const source = await readFile(join(directory, file), 'utf8')
    assert.doesNotMatch(source, /require\(["']electron["']\)/)
    for (const match of source.matchAll(/ipcRenderer\.(invoke|send)\("([^"]+)"/g)) {
      assert.ok((match[1] === 'invoke' ? registered : events).has(match[2]), `${file}: missing ${match[2]}`)
    }
  }
  assert.ok(registered.size > 80, 'full application contract must be registered')
  await assert.rejects(client.request('invoke', { windowId: 999, channel: 'application:confirm-close', args: ['keep'] }), /Unknown native window/)
})
