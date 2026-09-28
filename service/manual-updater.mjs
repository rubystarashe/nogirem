import { EventEmitter } from 'node:events'

// Electron update packages cannot install a Dioxus executable. Until a Dioxus
// release feed is supplied, fail explicitly; never install an Electron build.
const autoUpdater = Object.assign(new EventEmitter(), {
  async checkForUpdates() {
    throw new Error('Dioxus 배포용 업데이트 공급자가 아직 구성되지 않았습니다')
  },
  async downloadUpdate() {
    throw new Error('Dioxus 배포용 업데이트 공급자가 아직 구성되지 않았습니다')
  },
  quitAndInstall() {
    throw new Error('Dioxus 배포용 설치 프로그램이 아직 구성되지 않았습니다')
  },
})
export default { autoUpdater }
