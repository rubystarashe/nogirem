import { createInterface } from 'node:readline'

// Dedicated stdin/stdout pipe, never a TCP listener or renderer-controlled port.
export function createTransport(input, output, { timeoutMs = 120_000 } = {}) {
  let sequence = 0
  let closed = false
  const pending = new Map()
  const handlers = new Map()
  const events = new Map()
  const lines = createInterface({ input, crlfDelay: Infinity })
  const send = message => {
    if (closed) throw new Error('Desktop connection is closed')
    output.write(`${JSON.stringify(message)}\n`)
  }
  lines.on('line', async line => {
    let message
    try { message = JSON.parse(line) } catch { return }
    if (message.type === 'response') {
      const request = pending.get(message.id)
      if (!request) return
      pending.delete(message.id)
      clearTimeout(request.timer)
      if (message.error) request.reject(new Error(message.error))
      else request.resolve(message.result)
    } else if (message.type === 'request') {
      try {
        const handler = handlers.get(message.method)
        if (!handler) throw new Error(`Unknown desktop method: ${message.method}`)
        const result = await handler(message.params)
        send({ type: 'response', id: message.id, result: result ?? null })
      } catch (error) {
        if (!closed) send({ type: 'response', id: message.id, error: error.message })
      }
    } else if (message.type === 'event') {
      for (const callback of events.get(message.name) ?? []) callback(message.params)
    }
  })
  lines.on('close', () => {
    closed = true
    for (const request of pending.values()) {
      clearTimeout(request.timer)
      request.reject(new Error('Desktop disconnected'))
    }
    pending.clear()
    for (const callback of events.get('disconnect') ?? []) callback()
  })
  return {
    handle(method, handler) {
      if (handlers.has(method)) throw new Error(`Duplicate desktop method: ${method}`)
      handlers.set(method, handler)
    },
    on(name, callback) {
      const callbacks = events.get(name) ?? new Set()
      callbacks.add(callback)
      events.set(name, callbacks)
      return () => callbacks.delete(callback)
    },
    notify(name, params) { send({ type: 'event', name, params }) },
    request(method, params) {
      if (closed) return Promise.reject(new Error('Desktop connection is closed'))
      const id = ++sequence
      return new Promise((resolve, reject) => {
        const timer = setTimeout(() => {
          pending.delete(id)
          reject(new Error(`Desktop request timed out: ${method}`))
        }, timeoutMs)
        pending.set(id, { resolve, reject, timer })
        try { send({ type: 'request', id, method, params }) }
        catch (error) { clearTimeout(timer); pending.delete(id); reject(error) }
      })
    },
    close() { lines.close() },
  }
}
