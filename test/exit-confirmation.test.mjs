import test from "node:test"
import assert from "node:assert/strict"
import { assessExitConfirmation } from "../src/exit-confirmation.mjs"

test("실제로 적용 중인 프로세스 affinity가 있으면 종료 확인을 표시한다", () => {
  assert.deepEqual(assessExitConfirmation({
    hasLiveAppliedAffinity: true,
    hasManagedNic: false,
  }), {
    confirm: true,
    removeAppliedMarker: false,
  })
})

test("affinity가 복원되고 NIC 기록만 남으면 확인 없이 기록을 유지한다", () => {
  assert.deepEqual(assessExitConfirmation({
    hasLiveAppliedAffinity: false,
    hasManagedNic: true,
  }), {
    confirm: false,
    removeAppliedMarker: false,
  })
})

test("적용 중인 affinity와 NIC가 모두 없으면 오래된 기록을 제거한다", () => {
  assert.deepEqual(assessExitConfirmation({
    hasLiveAppliedAffinity: false,
    hasManagedNic: false,
  }), {
    confirm: false,
    removeAppliedMarker: true,
  })
})
