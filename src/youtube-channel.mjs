import { mkdir, readFile, rename, writeFile } from "node:fs/promises"
import { dirname } from "node:path"

const DAY_MS = 24 * 60 * 60 * 1000
const channelUrl = "https://www.youtube.com/channel/UCb7m0UV734CHm78Mb0zEBHg"

function findChannelEntity(value) {
  if (!value || typeof value !== "object") return null
  if (
    typeof value.name === "string"
    && typeof value.alternateName === "string"
    && typeof value.image === "string"
    && Array.isArray(value.interactionStatistic)
  ) {
    return value
  }
  for (const child of Object.values(value)) {
    if (Array.isArray(child)) {
      for (const item of child) {
        const found = findChannelEntity(item)
        if (found) return found
      }
    } else {
      const found = findChannelEntity(child)
      if (found) return found
    }
  }
  return null
}

function parseCount(value) {
  const count = Number(String(value ?? "").replaceAll(",", ""))
  return Number.isFinite(count) ? count : null
}

export function parseYouTubeChannelPage(html) {
  const scripts = [...html.matchAll(
    /<script[^>]+type=["']application\/ld\+json["'][^>]*>([\s\S]*?)<\/script>/gi,
  )]
  let channel = null
  for (const match of scripts) {
    try {
      channel = findChannelEntity(JSON.parse(match[1]))
      if (channel) break
    } catch {
    }
  }
  if (!channel) throw new Error("YouTube 채널 프로필 정보를 찾을 수 없습니다")

  const subscriberCount = parseCount(
    channel.interactionStatistic.find(statistic => (
      statistic?.interactionType?.["@type"] === "FollowAction"
    ))?.userInteractionCount,
  )
  const videoMatch = html.match(/"content":"동영상\s*([\d,]+)개"/)
    ?? html.match(/"content":"([\d,]+)\s+videos?"/i)
  const videoCount = parseCount(videoMatch?.[1])

  return {
    name: channel.name,
    handle: channel.alternateName,
    subscriberCount,
    videoCount,
    avatarUrl: channel.image.replaceAll("\\u003d", "="),
    channelUrl,
  }
}

async function readCache(cachePath) {
  try {
    return JSON.parse(await readFile(cachePath, "utf8"))
  } catch (error) {
    if (error.code === "ENOENT") return null
    throw error
  }
}

async function writeCache(cachePath, value) {
  await mkdir(dirname(cachePath), { recursive: true })
  const temporaryPath = `${cachePath}.${process.pid}.tmp`
  await writeFile(temporaryPath, JSON.stringify(value), "utf8")
  await rename(temporaryPath, cachePath)
}

async function fetchAvatarDataUrl(url, fetchImpl) {
  const response = await fetchImpl(url)
  if (!response.ok) throw new Error(`YouTube 프로필 이미지 조회 실패: HTTP ${response.status}`)
  const contentType = response.headers.get("content-type") ?? "image/jpeg"
  const bytes = Buffer.from(await response.arrayBuffer())
  if (bytes.length > 1024 * 1024) throw new Error("YouTube 프로필 이미지가 너무 큽니다")
  return `data:${contentType};base64,${bytes.toString("base64")}`
}

export async function getYouTubeChannelProfile({
  cachePath,
  fetchImpl = globalThis.fetch,
  now = Date.now(),
} = {}) {
  const cache = await readCache(cachePath)
  if (cache?.checkedAt && now - cache.checkedAt < DAY_MS) {
    return { ...cache, source: "cache", stale: false }
  }

  try {
    const response = await fetchImpl(`${channelUrl}?hl=ko`, {
      headers: {
        "Accept-Language": "ko-KR,ko;q=0.9,en;q=0.8",
        "User-Agent": "Mozilla/5.0",
      },
    })
    if (!response.ok) throw new Error(`YouTube 채널 조회 실패: HTTP ${response.status}`)
    const profile = parseYouTubeChannelPage(await response.text())
    const avatarDataUrl = await fetchAvatarDataUrl(profile.avatarUrl, fetchImpl)
    const nextCache = {
      ...profile,
      avatarDataUrl,
      checkedAt: now,
    }
    await writeCache(cachePath, nextCache)
    return { ...nextCache, source: "network", stale: false }
  } catch (error) {
    if (cache) {
      return {
        ...cache,
        source: "cache",
        stale: true,
        error: error.message,
      }
    }
    throw error
  }
}
