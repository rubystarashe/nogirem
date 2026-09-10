import assert from "node:assert/strict"
import test from "node:test"
import { readFile } from "node:fs/promises"

const helperBuildFiles = await Promise.all([
  "../native/radeon-helper/CMakeLists.txt",
  "../native/recorder-helper/CMakeLists.txt",
  "../native/input-guard-helper/CMakeLists.txt",
].map(path => readFile(new URL(path, import.meta.url), "utf8")))

test("설치본의 C++ helper는 외부 VC++ 런타임 없이 실행된다", () => {
  for (const buildFile of helperBuildFiles) {
    assert.match(
      buildFile,
      /set\(CMAKE_MSVC_RUNTIME_LIBRARY "MultiThreaded\$<\$<CONFIG:Debug>:Debug>"\)/,
    )
  }
})
