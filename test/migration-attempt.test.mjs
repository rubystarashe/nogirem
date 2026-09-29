import assert from "node:assert/strict"
import test from "node:test"
import { createMigrationAttemptGate } from "../src/migration-attempt.mjs"

function child() {
  return {
    killed: false,
    killCalls: 0,
    kill() {
      this.killed = true
      this.killCalls += 1
    },
  }
}

test("시간 초과 뒤 새 시도는 이전 helper를 종료하고 늦은 콜백을 무시한다", () => {
  const gate = createMigrationAttemptGate()
  const first = child()
  const second = child()
  gate.replace("first")
  assert.equal(gate.attach("first", first), true)
  assert.equal(gate.stop("first"), true)
  assert.equal(gate.isCurrent("first"), false)
  assert.equal(gate.current(), "")
  gate.replace("second")
  assert.equal(first.killCalls, 1)
  assert.equal(gate.attach("second", second), true)
  assert.equal(gate.isCurrent("first"), false)
  assert.equal(gate.isCurrent("second"), true)
  assert.equal(gate.stop("first"), false)
  assert.equal(second.killCalls, 0)
})

test("시간 초과로 무효화된 시도의 지연 refresh와 exit는 현재 상태를 바꾸지 못한다", () => {
  const gate = createMigrationAttemptGate()
  gate.replace("timed-out")
  assert.equal(gate.stop("timed-out"), true)
  assert.equal(gate.isCurrent("timed-out"), false)
  assert.equal(gate.clear("timed-out"), undefined)
  assert.equal(gate.current(), "")
})

test("새 시도 이후 도착한 이전 helper 연결은 즉시 종료한다", () => {
  const gate = createMigrationAttemptGate()
  const stale = child()
  gate.replace("first")
  gate.replace("second")
  assert.equal(gate.attach("first", stale), false)
  assert.equal(stale.killCalls, 1)
})
