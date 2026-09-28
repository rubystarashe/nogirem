(() => {
  window.__nogiremErrors = [];
  window.addEventListener('error', event => { if (event.message) window.__nogiremErrors.push(event.message); });
  window.addEventListener('unhandledrejection', event => window.__nogiremErrors.push(String(event.reason)));
  const pending = new Map(), listeners = new Map(), queue = [];
  let sequence = 0;
  const send = message => window.__nogiremSend ? window.__nogiremSend(message) : queue.push(message);
  window.close = () => send({ close: true });
  document.addEventListener('pointerdown', event => {
    if (event.button !== 0 || window.characterGuide || window.dxvkGuide) return;
    if (!event.target.closest?.('button,input,select,a') && event.target.closest?.('.titlebar,.window-drag,.drag-region')) send({ drag: true });
  });
  document.addEventListener('keydown', event => {
    if (event.key === 'F5' || ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'r')) event.preventDefault();
  });
  document.addEventListener('click', event => {
    const link = event.target.closest?.('a[href]');
    if (link && /^https?:/.test(link.href) && new URL(link.href).origin !== location.origin) {
      event.preventDefault(); send({ external: link.href });
    }
  });
  window.__nogiremDispatch = (channel, args) => {
    for (const callback of listeners.get(channel) ?? []) callback({}, ...args);
  };
  window.__nogiremAttach = nativeSend => {
    window.__nogiremSend = nativeSend;
    for (const message of queue.splice(0)) nativeSend(message);
  };
  window.__nogiremReply = message => {
    const entry = pending.get(message.id);
    if (!entry) return;
    pending.delete(message.id);
    clearTimeout(entry.timer);
    if (message.error) entry.reject(new Error(message.error));
    else entry.resolve(message.result);
  };
  window.__nogiremBridge = {
    contextBridge: { exposeInMainWorld: (name, api) => {
      Object.defineProperty(window, name, { value: Object.freeze(api), writable: false });
    } },
    ipcRenderer: {
      invoke(channel, ...args) {
        const id = ++sequence;
        return new Promise((resolve, reject) => {
          const timer = setTimeout(() => { pending.delete(id); reject(new Error('요청 시간이 초과되었습니다')); }, 240000);
          pending.set(id, { resolve, reject, timer });
          send({ id, channel, args });
        });
      },
      send(channel, ...args) { send({ channel, args, notify: true }); },
      on(channel, callback) {
        const callbacks = listeners.get(channel) ?? new Set();
        callbacks.add(callback); listeners.set(channel, callbacks);
      },
      removeListener(channel, callback) { listeners.get(channel)?.delete(callback); },
    },
  };
})();
