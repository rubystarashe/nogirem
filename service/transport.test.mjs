import test from 'node:test'
import assert from 'node:assert/strict'
import { PassThrough } from 'node:stream'
import { createTransport } from './transport.mjs'

function pair(timeoutMs = 500) {
  const a = new PassThrough(), b = new PassThrough()
  return [createTransport(a, b, { timeoutMs }), createTransport(b, a, { timeoutMs })]
}
test('concurrent responses stay correlated even when resolved in reverse order', async () => {
  const [client, server] = pair()
  let finish
  server.handle('slow', () => new Promise(resolve => { finish = resolve }))
  server.handle('fast', value => value)
  const slow = client.request('slow')
  assert.equal(await client.request('fast', 'second'), 'second')
  finish('first')
  assert.equal(await slow, 'first')
  client.close(); server.close()
})
test('unknown methods and backend errors are rejected without hanging', async () => {
  const [client, server] = pair()
  server.handle('fail', () => { throw new Error('access denied') })
  await assert.rejects(client.request('missing'), /Unknown desktop method/)
  await assert.rejects(client.request('fail'), /access denied/)
  client.close(); server.close()
})
test('a missing reply times out', async () => {
  const [client, server] = pair(20)
  server.handle('wait', () => new Promise(() => {}))
  await assert.rejects(client.request('wait'), /timed out/)
  client.close(); server.close()
})
test('disconnect rejects outstanding requests', async () => {
  const [client, server] = pair()
  server.handle('wait', () => new Promise(() => {}))
  const pending = client.request('wait')
  client.close()
  await assert.rejects(pending, /disconnected/)
  server.close()
})
