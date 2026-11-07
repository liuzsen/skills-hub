import { strToU8, zipSync } from 'fflate'
import type { SkillFile, SkillSummary } from './mock-data'

export function downloadMockZip(skill: SkillSummary, files: SkillFile[]) {
  const entries = Object.fromEntries(
    files.map((file) => [`${skill.id}/${file.path}`, strToU8(file.content)]),
  )
  const bytes = zipSync(entries)
  const blob = new Blob([new Uint8Array(bytes)], { type: 'application/zip' })
  const url = URL.createObjectURL(blob)
  const link = document.createElement('a')
  link.href = url
  link.download = `${skill.id}-latest-mock.zip`
  document.body.append(link)
  link.click()
  link.remove()
  window.setTimeout(() => URL.revokeObjectURL(url), 1000)
}
