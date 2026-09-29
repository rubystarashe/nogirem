function stop(child) {
  if (!child || child.killed) return
  try {
    child.kill()
  } catch {}
}

export function createMigrationAttemptGate() {
  let attempt = ""
  let child = null
  return {
    current: () => attempt,
    isCurrent: candidate => candidate === attempt,
    replace(candidate) {
      stop(child)
      attempt = candidate
      child = null
    },
    attach(candidate, process) {
      if (candidate !== attempt) {
        stop(process)
        return false
      }
      child = process
      return true
    },
    clear(candidate) {
      if (candidate === attempt) child = null
    },
    stop(candidate) {
      if (candidate !== attempt) return false
      stop(child)
      attempt = ""
      child = null
      return true
    },
  }
}
