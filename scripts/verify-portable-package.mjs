import { createHash, createPublicKey, verify } from 'node:crypto'
import { readFile } from 'node:fs/promises'
import { basename, dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import AdmZip from 'adm-zip'

const root = dirname(dirname(fileURLToPath(import.meta.url)))
const [archivePath, expectedVersion, option] = process.argv.slice(2)
if (!archivePath || !/^\d+\.\d+\.\d+$/.test(expectedVersion)) {
  throw new Error('Usage: node scripts/verify-portable-package.mjs <archive> <version>')
}
if (basename(archivePath) !== `nogirem-dioxus-portable-${expectedVersion}.zip`) {
  throw new Error('Unexpected portable archive filename')
}

const archive = new AdmZip(archivePath)
const allEntries = archive.getEntries()
for (const entry of allEntries) {
  const mode = (entry.attr >>> 16) & 0o170000
  if (mode === 0o120000) throw new Error(`Portable archive link is not allowed: ${entry.entryName}`)
}
const entries = allEntries.filter(entry => !entry.isDirectory)
const names = entries.map(entry => entry.entryName.replaceAll('\\', '/')).sort()
for (const name of names) {
  const segments = name.split('/')
  const special = segments.some(segment => !segment || segment.includes(':') || /[ .]$/.test(segment) || /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(?:\.|$)/i.test(segment))
  if (!name || name.startsWith('/') || name.includes('../') || /^[A-Za-z]:/.test(name) || special) {
    throw new Error(`Unsafe portable archive path: ${name}`)
  }
}
const manifestEntry = archive.getEntry('portable-manifest.json')
if (!manifestEntry) throw new Error('Missing portable-manifest.json')
const manifestBytes = manifestEntry.getData()
const manifest = JSON.parse(manifestBytes.toString('utf8'))
if (manifest.schemaVersion !== 1 || manifest.version !== expectedVersion) {
  throw new Error('Portable manifest version mismatch')
}
const signatureEntry = archive.getEntry('portable-manifest.sig')
const expected = [...Object.keys(manifest.files), 'portable-manifest.json', ...(signatureEntry ? ['portable-manifest.sig'] : [])].sort()
if (JSON.stringify(names) !== JSON.stringify(expected)) throw new Error('Portable archive file list mismatch')
for (const [name, hash] of Object.entries(manifest.files)) {
  const entry = archive.getEntry(name)
  if (!entry) throw new Error(`Missing portable file: ${name}`)
  const actual = createHash('sha256').update(entry.getData()).digest('hex')
  if (actual !== hash) throw new Error(`Portable file hash mismatch: ${name}`)
}
if (!archive.getEntry('portable.marker') || !archive.getEntry('nogirem.exe')) {
  throw new Error('Portable runtime marker or executable missing')
}
if (option === '--require-signature') {
  if (!signatureEntry) throw new Error('Missing portable manifest signature')
  const cfg = JSON.parse(await readFile(join(root, 'build-config/update.json'), 'utf8'))
  const publicKey = createPublicKey({ key: Buffer.concat([Buffer.from('302a300506032b6570032100', 'hex'), Buffer.from(cfg.publicKey, 'base64')]), type: 'spki', format: 'der' })
  const signature = Buffer.from(signatureEntry.getData().toString('utf8').trim(), 'base64')
  if (!verify(null, manifestBytes, publicKey, signature)) throw new Error('Invalid portable manifest signature')
}
console.log(JSON.stringify({ archive: archivePath, version: expectedVersion, files: names.length, verified: true }))
