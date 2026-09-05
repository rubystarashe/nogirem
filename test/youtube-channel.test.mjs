import test from "node:test"
import assert from "node:assert/strict"
import { mkdtemp, rm } from "node:fs/promises"
import { tmpdir } from "node:os"
import { join } from "node:path"
import {
  getYouTubeChannelProfile,
  parseYouTubeChannelPage,
} from "../src/youtube-channel.mjs"

const channelHtml = `
<script type="application/ld+json">
{
  "@type": "ProfilePage",
  "mainEntity": {
    "@type": "Person",
    "name": "마비노기 렘",
    "alternateName": "@마비노기렘",
    "image": "https://example.com/avatar.jpg",
    "interactionStatistic": [
      {
        "@type": "InteractionCounter",
        "interactionType": { "@type": "FollowAction" },
        "userInteractionCount": "152"
      }
    ]
  }
}
</script>
<script>{"content":"동영상 38개"}</script>
`

test("YouTube 채널 공개 정보에서 프로필과 통계를 읽는다", () => {
  const profile = parseYouTubeChannelPage(channelHtml)
  assert.equal(profile.name, "마비노기 렘")
  assert.equal(profile.handle, "@마비노기렘")
  assert.equal(profile.subscriberCount, 152)
  assert.equal(profile.videoCount, 38)
  assert.equal(profile.avatarUrl, "https://example.com/avatar.jpg")
})

test("YouTube 채널 정보는 하루 동안 캐시를 재사용한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-youtube-"))
  const cachePath = join(directory, "channel.json")
  let requestCount = 0
  const fetchImpl = async url => {
    requestCount++
    if (String(url).includes("youtube.com")) {
      return new Response(channelHtml, {
        status: 200,
        headers: { "content-type": "text/html; charset=utf-8" },
      })
    }
    return new Response(Buffer.from([1, 2, 3]), {
      status: 200,
      headers: { "content-type": "image/jpeg" },
    })
  }

  try {
    const first = await getYouTubeChannelProfile({
      cachePath,
      fetchImpl,
      now: 100000,
    })
    const second = await getYouTubeChannelProfile({
      cachePath,
      fetchImpl,
      now: 101000,
    })
    assert.equal(first.source, "network")
    assert.equal(second.source, "cache")
    assert.equal(second.subscriberCount, 152)
    assert.equal(requestCount, 2)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})

test("YouTube 파싱 실패 시 만료된 마지막 캐시를 사용한다", async () => {
  const directory = await mkdtemp(join(tmpdir(), "nogirem-youtube-stale-"))
  const cachePath = join(directory, "channel.json")
  const fetchImpl = async url => {
    if (String(url).includes("youtube.com")) {
      return new Response(channelHtml, { status: 200 })
    }
    return new Response(Buffer.from([1, 2, 3]), {
      status: 200,
      headers: { "content-type": "image/jpeg" },
    })
  }

  try {
    await getYouTubeChannelProfile({
      cachePath,
      fetchImpl,
      now: 100000,
    })
    const fallback = await getYouTubeChannelProfile({
      cachePath,
      fetchImpl: async () => new Response("<html>변경된 구조</html>", { status: 200 }),
      now: 100000 + 25 * 60 * 60 * 1000,
    })
    assert.equal(fallback.source, "cache")
    assert.equal(fallback.stale, true)
    assert.equal(fallback.name, "마비노기 렘")
    assert.match(fallback.error, /프로필 정보를 찾을 수 없습니다/)
  } finally {
    await rm(directory, { recursive: true, force: true })
  }
})
