// The host validates with the reference's actual encrypted models. The networkless browser
// process then checks a receipt bound to those exact seed bytes and validator implementation.
import fs from 'node:fs'
import path from 'node:path'
import { createHash } from 'node:crypto'
import { spawnSync } from 'node:child_process'
import { PARITY_DIR, REPO_DIR } from './config.ts'
const validator = path.join(REPO_DIR, 'reference-tools/campfire/verify_parity_seed.rb')

export function seedFingerprint(seed: string, seedDir: string): string {
  const hash = createHash('sha256').update(seed).update(fs.readFileSync(validator))
  const root = path.join(seedDir, seed)
  const visit = (dir: string) => {
    for (const entry of fs.readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      const file = path.join(dir, entry.name)
      if (entry.isDirectory()) visit(file)
      else if (entry.isFile()) hash.update(path.relative(root, file)).update(fs.readFileSync(file))
      else throw new Error(`seed validation: unsupported file ${file}`)
    }
  }
  visit(root)
  return hash.digest('hex')
}

export function validateSeed(seed: string, seedDir: string, time: string, receipt?: string): void {
  const fingerprint = seedFingerprint(seed, seedDir)
  const supplied = process.env.PARITY_SEED_VALIDATION_FILE
  if (!receipt && supplied) {
    const result = JSON.parse(fs.readFileSync(supplied, 'utf8'))
    if (result.seed !== seed || result.time !== time || result.fingerprint !== fingerprint) throw new Error('seed validation receipt does not match the seed, clock or validator')
    return
  }
  const result = spawnSync(path.join(PARITY_DIR, 'bin/reference'), ['runner', '--seed', seed, '--time', time, '--freeze', validator, seed], {
    env: { ...process.env, PARITY_SEED_DIR: seedDir }, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024,
  })
  if (result.status !== 0) throw new Error(`seed validation failed for ${seed}: ${result.error ?? result.stderr}\n${result.stdout}`)
  if (seedFingerprint(seed, seedDir) !== fingerprint) throw new Error('seed changed during validation')
  if (receipt) fs.writeFileSync(receipt, JSON.stringify({ seed, time, fingerprint, verification: JSON.parse(result.stdout) }))
  console.error(`seed validation [${seed}]: ${result.stdout.trim()}`)
}
