import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'

interface Find {
  id: string
  finder: string
  item_name: string
  note: string | null
}
interface NotifyPayload {
  corner: string
  finds: Find[]
}

const DISMISS_MS = 9000

export default function Toast() {
  const [items, setItems] = useState<Find[]>([])
  const [top, setTop] = useState(false)
  const timers = useRef<Record<string, number>>({})

  function dismiss(id: string) {
    window.clearTimeout(timers.current[id])
    delete timers.current[id]
    setItems((prev) => {
      const next = prev.filter((i) => i.id !== id)
      if (next.length === 0) invoke('hide_toast').catch(() => {})
      return next
    })
  }

  useEffect(() => {
    const unlisten = listen<NotifyPayload>('notify', (e) => {
      const { corner, finds } = e.payload
      setTop((corner || '').startsWith('top'))
      setItems((prev) => {
        const have = new Set(prev.map((i) => i.id))
        return [...prev, ...(finds || []).filter((f) => !have.has(f.id))]
      })
      for (const f of finds || []) {
        if (!timers.current[f.id]) {
          timers.current[f.id] = window.setTimeout(() => dismiss(f.id), DISMISS_MS)
        }
      }
    })
    return () => {
      unlisten.then((f) => f())
      Object.values(timers.current).forEach((t) => window.clearTimeout(t))
    }
  }, [])

  if (items.length === 0) return null

  return (
    <div className={'toasts' + (top ? ' top' : '')}>
      {items.map((f) => (
        <div className="toast" key={f.id} onClick={() => dismiss(f.id)} title="Click to dismiss">
          <div className="toast-head">🔔 {f.finder} found your item</div>
          <div className="toast-item">{f.item_name}</div>
          {f.note && <div className="toast-note">"{f.note}"</div>}
          <div className="toast-hint">Confirm it on the website · click to dismiss</div>
        </div>
      ))}
    </div>
  )
}
