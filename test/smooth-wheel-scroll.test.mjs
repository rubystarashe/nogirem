import assert from "node:assert/strict"
import test from "node:test"
import { createSmoothWheelScroller } from "../web/smooth-wheel-scroll.mjs"

function createFixture() {
  const element = {
    scrollHeight: 2000,
    clientHeight: 400,
    scrollTop: 0,
  }
  let callback = null
  let cancelled = null
  let now = 0
  const scroller = createSmoothWheelScroller(
    () => element,
    {
      requestFrame: next => {
        callback = next
        return 1
      },
      cancelFrame: frame => {
        cancelled = frame
      },
      currentTime: () => now,
    },
  )
  const wheel = (deltaY, deltaMode = 0) => {
    let prevented = false
    scroller.handleWheel({
      ctrlKey: false,
      deltaMode,
      deltaY,
      preventDefault: () => {
        prevented = true
      },
    })
    return prevented
  }
  const draw = time => {
    now = time
    const next = callback
    callback = null
    next?.(time)
  }
  return { element, scroller, wheel, draw, cancelled: () => cancelled }
}

test("부드러운 스크롤은 정상 프레임에서 시간 비례로 이동한다", () => {
  const fixture = createFixture()
  assert.equal(fixture.wheel(100), true)
  fixture.draw(1000 / 60)
  assert.ok(fixture.element.scrollTop > 14)
  assert.ok(fixture.element.scrollTop < 15)
})

test("휠 입력 누적 거리는 화면 높이 이내로 제한한다", () => {
  const fixture = createFixture()
  for (let index = 0; index < 20; index++) fixture.wheel(100)
  fixture.draw(150)
  assert.equal(fixture.element.scrollTop, 260)
})

test("렌더러 프레임이 오래 지연되면 남은 스크롤을 즉시 보정한다", () => {
  const fixture = createFixture()
  fixture.wheel(100)
  fixture.draw(120)
  assert.equal(fixture.element.scrollTop, 80)
  fixture.scroller.stop()
  assert.equal(fixture.cancelled(), null)
})
