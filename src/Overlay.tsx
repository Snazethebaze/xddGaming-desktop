import { useEffect, useRef, useState } from 'react'
import { listen } from '@tauri-apps/api/event'
import { invoke } from '@tauri-apps/api/core'
import type { LookupPayload, Match } from './types'

// Close if the user never interacts; if they do (hover), keep it and only close
// shortly after the pointer leaves the panel.
const IDLE_MS = 6000
const LEAVE_MS = 2500

type FoundState = 'idle' | 'sending' | 'done' | 'dup' | 'err'

export default function Overlay() {
  const [p, setP] = useState<LookupPayload | null>(null)
  const [found, setFound] = useState<Record<string, FoundState>>({})
  const [foot, setFoot] = useState<{ ok: boolean; text: string } | null>(null)
  const [noteOpen, setNoteOpen] = useState<Record<string, boolean>>({})
  const [noteText, setNoteText] = useState<Record<string, string>>({})
  const timer = useRef<number | undefined>(undefined)

  function clearTimer() {
    window.clearTimeout(timer.current)
  }
  function scheduleClose(ms: number) {
    clearTimer()
    timer.current = window.setTimeout(close, ms)
  }
  function close() {
    invoke('close_overlay').catch(() => {})
  }

  useEffect(() => {
    const unlisten = listen<LookupPayload>('lookup', (e) => {
      setP(e.payload)
      setFound({}) // fresh item — reset any per-row state
      setFoot(null)
      setNoteOpen({})
      setNoteText({})
      if (e.payload.state === 'pending') clearTimer()
      else scheduleClose(IDLE_MS)
    })
    return () => {
      unlisten.then((f) => f())
      clearTimer()
    }
  }, [])

  function toggleNote(id: string) {
    invoke('focus_overlay').catch(() => {}) // so the field can receive typing
    setNoteOpen((o) => ({ ...o, [id]: !o[id] }))
  }

  async function markFound(m: Match) {
    setFound((f) => ({ ...f, [m.want_id]: 'sending' }))
    setFoot(null)
    clearTimer() // interacting — don't dismiss mid-action
    try {
      const note = (noteText[m.want_id] ?? '').trim()
      const res = await invoke<string>('mark_found', { wantId: m.want_id, note: note || null })
      const dup = res === 'duplicate'
      setFound((f) => ({ ...f, [m.want_id]: dup ? 'dup' : 'done' }))
      setNoteOpen((o) => ({ ...o, [m.want_id]: false }))
      setFoot({ ok: true, text: dup ? `Already told ${m.who}.` : `Sent to ${m.who} ✓` })
    } catch (err) {
      setFound((f) => ({ ...f, [m.want_id]: 'err' }))
      setFoot({ ok: false, text: String(err) })
    }
  }

  if (!p) return null

  return (
    <div className="panel" onMouseEnter={clearTimer} onMouseLeave={() => scheduleClose(LEAVE_MS)}>
      <div className="panel-head">
        <span className="panel-title">{p.state === 'pending' ? 'Wishlist' : p.item || 'Wishlist'}</span>
        <span className="panel-close" onClick={close} title="Close">
          ×
        </span>
      </div>

      {p.state === 'result' && (
        <>
          <div className="panel-sub">
            {p.matches.length} {p.matches.length === 1 ? 'person wants' : 'people want'} this
          </div>
          <ul className="reqs">
            {p.matches.map((m) => {
              const mine = m.owner_id === p.me
              const st = found[m.want_id] ?? 'idle'
              const label =
                st === 'sending' ? '…' : st === 'done' ? 'Found ✓' : st === 'dup' ? 'Already ✓' : st === 'err' ? 'Retry' : 'Found'
              return (
                <li className="req" key={m.want_id}>
                  <span className="req-av">{m.who.charAt(0).toUpperCase()}</span>
                  <span className="req-name">
                    {m.who}
                    {mine && <span className="req-you">you</span>}
                  </span>
                  <span className={'badge prio-' + m.priority}>{m.priorityLabel}</span>
                  {!mine && st !== 'done' && st !== 'dup' && (
                    <button
                      className={'note-toggle' + (noteOpen[m.want_id] ? ' on' : '')}
                      onClick={() => toggleNote(m.want_id)}
                      title="Add an optional note"
                    >
                      ✎
                    </button>
                  )}
                  {!mine && (
                    <button
                      className={'found-btn ' + st}
                      disabled={st === 'sending' || st === 'done' || st === 'dup'}
                      onClick={() => markFound(m)}
                    >
                      {label}
                    </button>
                  )}
                  {m.note && <span className="req-note">"{m.note}"</span>}
                  {!mine && noteOpen[m.want_id] && (
                    <input
                      className="note-input"
                      autoFocus
                      placeholder="optional note — press Enter to send"
                      value={noteText[m.want_id] ?? ''}
                      onChange={(e) => setNoteText((t) => ({ ...t, [m.want_id]: e.target.value }))}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') markFound(m)
                      }}
                    />
                  )}
                </li>
              )
            })}
          </ul>
          {foot && <div className={'panel-foot' + (foot.ok ? '' : ' err')}>{foot.text}</div>}
        </>
      )}

      {p.state === 'pending' && (
        <div className="panel-msg">
          <div className="spinner" />
          Checking the wishlist…
        </div>
      )}

      {p.state === 'empty' && (
        <div className="panel-msg">
          {p.message ? p.message : p.item ? `No one is wishlisting ${p.item}.` : 'No one is wishlisting this.'}
        </div>
      )}

      {p.state === 'error' && <div className="panel-msg err">{p.message || 'Something went wrong.'}</div>}
    </div>
  )
}
