import test from 'node:test'
import assert from 'node:assert/strict'
import { mkdtemp, writeFile, unlink, rmdir } from 'node:fs/promises'
import { join } from 'node:path'
import { tmpdir } from 'node:os'
import { localVideoResponse, maximumMediaChunkBytes } from './media-response.mjs'

test('video ranges are bounded, seekable and correctly describe the complete recording', async t => {
  const directory = await mkdtemp(join(tmpdir(), 'nogirem-media-test-'))
  const file = join(directory, 'fixture.mp4')
  const size = maximumMediaChunkBytes + 100
  await writeFile(file, Buffer.alloc(size, 7))
  t.after(async () => { await unlink(file); await rmdir(directory) })
  const request = (range, method = 'GET') => new Request('http://local/video', { method, headers: range ? { range } : {} })
  const initial = await localVideoResponse(file, request())
  assert.equal(initial.status, 206)
  assert.equal(initial.headers.get('content-range'), `bytes 0-${maximumMediaChunkBytes-1}/${size}`)
  assert.equal((await initial.arrayBuffer()).byteLength, maximumMediaChunkBytes)
  const suffix = await localVideoResponse(file, request('bytes=-64'))
  assert.equal((await suffix.arrayBuffer()).byteLength, 64)
  assert.equal(suffix.headers.get('content-range'), `bytes ${size-64}-${size-1}/${size}`)
  for (const invalid of ['bytes=-0', 'bytes=8-2', `bytes=${size}-`, 'bytes=0-1,3-4']) {
    assert.equal((await localVideoResponse(file, request(invalid))).status, 416)
  }
  const head = await localVideoResponse(file, request(null, 'HEAD'))
  assert.equal(head.headers.get('content-length'), String(size))
  assert.equal((await head.arrayBuffer()).byteLength, 0)
})
