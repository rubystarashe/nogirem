# 포터블 보강 검증 근거

- created_at: `2026-09-29T16:05:00+09:00`
- target_commit: `11dc9c671a919edb5826c69239a468148cf3bba2`
- package: `release/dioxus-0.4.1-2026-09-29T06-59-43-740Z`

## 검사 결과

- `npm test`: 192 passed
- `cargo test --locked --manifest-path desktop/backend/Cargo.toml`: 53 passed, 공개 실다운로드 1 ignored
- `node --test test/portable-release.test.mjs`: 5 passed
- `node scripts/package-dioxus.mjs`: 설치형·포터블 NSIS 생성과 Ed25519 매니페스트 자체 검증 통과
- 최종 `portableCacheId`: `d4d7aff2a8674936289b5c13d76cd519927f228282a9ecb70ad658091863b38c`

## 공개 자산

- 설치형 SHA-256: `d8a907103614bd9ad9572205e6db9b7b0882a58d2035f5266c9576ae823bcf7a`
- 포터블 SHA-256: `9628882acc2527742eeedb0beed378a3dda192e4984d56c5d21704e3ab96b033`
- `update.json` SHA-256: `c443c9eacab3e4c90756d1611bc09b6f838c0fc762081008565a1a41cd1c2995`
- `portable-update.json` SHA-256: `967119633d3d60e53aa7e75776d2712fce95506816c0848bdf9990b21826547f`
- GitHub Release digest가 네 로컬 SHA-256과 모두 일치
- 공개 설치형 blocking 다운로드 검사 통과
- 공개 포터블 다운로드 크기·SHA-256 검사 통과

## 알려진 비관련 실패

- `npm run test:desktop`: 8개 중 7개 통과, 기존 Markdown fixture가 `342.0MiB`를 찾지 못해 1개 실패
