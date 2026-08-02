import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import type { PeopleResult, Person, Settings as S } from './types'

// Turn a KeyboardEvent into a Tauri accelerator string (e.g. "Alt+Shift+W").
function comboFrom(e: KeyboardEvent): string | null {
  const mods: string[] = []
  if (e.ctrlKey) mods.push('Ctrl')
  if (e.altKey) mods.push('Alt')
  if (e.shiftKey) mods.push('Shift')
  if (e.metaKey) mods.push('Super')

  const code = e.code
  let key = ''
  if (code.startsWith('Key')) key = code.slice(3)
  else if (code.startsWith('Digit')) key = code.slice(5)
  else if (/^F\d{1,2}$/.test(code)) key = code
  else if (code === 'Space') key = 'Space'
  else if (code.startsWith('Arrow')) key = code.slice(5)
  else if (['Backquote', 'Minus', 'Equal', 'Comma', 'Period', 'Slash', 'Semicolon', 'Quote', 'BracketLeft', 'BracketRight', 'Backslash'].includes(code)) key = code

  if (!key) return null
  return [...mods, key].join('+')
}

type Tab = 'connection' | 'hotkey' | 'notifications'
const TABS: { id: Tab; label: string }[] = [
  { id: 'connection', label: 'Connection' },
  { id: 'hotkey', label: 'Hotkey' },
  { id: 'notifications', label: 'Alerts' },
]

export default function Settings() {
  const [initializing, setInitializing] = useState(true)
  const [connected, setConnected] = useState(false)
  const [connecting, setConnecting] = useState(false)
  const [connectErr, setConnectErr] = useState('')

  const [token, setToken] = useState('')
  const [league, setLeague] = useState('')
  const [people, setPeople] = useState<Person[]>([])
  const [identityId, setIdentityId] = useState('')
  const [hotkey, setHotkey] = useState('Alt+W')
  const [capturing, setCapturing] = useState(false)
  const [toastEnabled, setToastEnabled] = useState(true)
  const [toastSound, setToastSound] = useState(true)
  const [toastCorner, setToastCorner] = useState('bottom-right')
  const [pollSecs, setPollSecs] = useState(60)
  const [tab, setTab] = useState<Tab>('connection')

  async function connect(tok: string, keepIdentity: string) {
    setConnecting(true)
    setConnectErr('')
    try {
      const res = await invoke<PeopleResult>('list_people', { token: tok.trim() })
      setLeague(res.league)
      setPeople(res.people)
      setToken(tok.trim())
      setConnected(true)
      const id = res.people.some((p) => p.id === keepIdentity) ? keepIdentity : ''
      setIdentityId(id)
      setTab('connection')
    } catch (e) {
      setConnected(false)
      setConnectErr(String(e))
    } finally {
      setConnecting(false)
    }
  }

  // Load saved settings, then auto-connect if a token is stored.
  useEffect(() => {
    invoke<S>('get_settings')
      .then(async (s) => {
        setHotkey(s.hotkey || 'Alt+W')
        setIdentityId(s.identity_id || '')
        setToastEnabled(s.toast_enabled ?? true)
        setToastSound(s.toast_sound ?? true)
        setToastCorner(s.toast_corner || 'bottom-right')
        // Floor at 60s (matches the Rust clamp); migrates older 15/25/45s installs up.
        setPollSecs(Math.max(60, s.poll_secs || 60))
        setToken(s.token || '')
        if (s.token) await connect(s.token, s.identity_id || '')
      })
      .catch(() => {})
      .finally(() => setInitializing(false))
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  // Auto-apply: persist whenever a setting changes (no Save button).
  useEffect(() => {
    if (!connected) return
    const t = setTimeout(() => {
      const identityName = people.find((p) => p.id === identityId)?.display_name ?? ''
      invoke('update_settings', {
        token,
        hotkey,
        identityId,
        identityName,
        toastEnabled,
        toastSound,
        toastCorner,
        pollSecs,
      }).catch(() => {})
    }, 150)
    return () => clearTimeout(t)
  }, [connected, token, hotkey, identityId, toastEnabled, toastSound, toastCorner, pollSecs, people])

  useEffect(() => {
    if (!capturing) return
    const onKey = (e: KeyboardEvent) => {
      e.preventDefault()
      if (e.key === 'Escape') return setCapturing(false)
      const combo = comboFrom(e)
      if (combo) {
        setHotkey(combo)
        setCapturing(false)
      }
    }
    window.addEventListener('keydown', onKey)
    return () => window.removeEventListener('keydown', onKey)
  }, [capturing])

  if (initializing) {
    return (
      <div className="settings">
        <p className="lede">Loading…</p>
      </div>
    )
  }

  // ── Onboarding / reconnect: enter a token ────────────────────────────────
  if (!connected) {
    return (
      <div className="settings">
        <h1>PoE Wishlist Overlay</h1>
        <p className="lede">Paste your league token to get started — copy it from the top bar of the web app.</p>
        <div className="field">
          <label>League token</label>
          <input
            className="input"
            type="password"
            value={token}
            autoFocus
            onChange={(e) => setToken(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter' && token.trim()) connect(token, identityId)
            }}
            placeholder="paste token"
          />
          <div className="hint">The token decides which league you're on. Stored only on this PC.</div>
        </div>
        <button className="btn btn-primary" disabled={!token.trim() || connecting} onClick={() => connect(token, identityId)}>
          {connecting ? 'Connecting…' : 'Connect'}
        </button>
        {connectErr && <div className="status err">{connectErr}</div>}
      </div>
    )
  }

  // ── Connected: tabbed settings ───────────────────────────────────────────
  return (
    <div className="settings">
      <div className="tabbar">
        {TABS.map((t) => (
          <button key={t.id} className={'tab' + (tab === t.id ? ' active' : '')} onClick={() => setTab(t.id)}>
            {t.label}
            {t.id === 'connection' && !identityId && <span className="tab-dot" />}
          </button>
        ))}
      </div>

      {tab === 'connection' && (
        <>
          <div className="field">
            <label>Connected league</label>
            <div className="readonly-league">{league || '—'}</div>
            <div className="hint">Your league is set by the token. To switch leagues, use a different token.</div>
          </div>

          <div className="field">
            <label>Who are you?</label>
            <select className="input" value={identityId} onChange={(e) => setIdentityId(e.target.value)}>
              <option value="">— pick your name —</option>
              {people.map((p) => (
                <option key={p.id} value={p.id}>
                  {p.display_name}
                </option>
              ))}
            </select>
            <div className="hint">You appear here once you've posted at least one item on this league's board (via the website).</div>
          </div>

          <button
            className="btn"
            onClick={() => {
              setConnected(false)
              setConnectErr('')
            }}
          >
            Use a different token
          </button>
        </>
      )}

      {tab === 'hotkey' && (
        <div className="field">
          <label>Lookup hotkey</label>
          <div className="hotkey-row">
            <div className={'hotkey-display' + (capturing ? ' capturing' : '')}>{capturing ? 'Press a key combo…' : hotkey}</div>
            <button className="btn" onClick={() => setCapturing((c) => !c)}>
              {capturing ? 'Cancel' : 'Change'}
            </button>
          </div>
          <div className="hint">Global (works even outside PoE), so avoid common combos like Ctrl+C/X/V/W. Alt+W is a safe default.</div>
        </div>
      )}

      {tab === 'notifications' && (
        <>
          <label className="check" style={{ marginBottom: '14px' }}>
            <input type="checkbox" checked={toastEnabled} onChange={(e) => setToastEnabled(e.target.checked)} />
            <span>Pop a toast when someone finds one of my items</span>
          </label>
          {toastEnabled && (
            <>
              <label className="check" style={{ marginBottom: '14px' }}>
                <input type="checkbox" checked={toastSound} onChange={(e) => setToastSound(e.target.checked)} />
                <span>Play a sound with the toast</span>
              </label>
              <div className="field two">
                <div>
                  <label>Corner</label>
                  <select className="input" value={toastCorner} onChange={(e) => setToastCorner(e.target.value)}>
                    <option value="bottom-right">Bottom-right</option>
                    <option value="bottom-left">Bottom-left</option>
                    <option value="top-right">Top-right</option>
                    <option value="top-left">Top-left</option>
                  </select>
                </div>
                <div>
                  <label>Check every</label>
                  <select className="input" value={pollSecs} onChange={(e) => setPollSecs(Number(e.target.value))}>
                    <option value={60}>1 minute</option>
                    <option value={120}>2 minutes</option>
                    <option value={300}>5 minutes</option>
                  </select>
                </div>
              </div>
              <div className="hint">Only checks while PoE is running (so it costs nothing when you're not playing).</div>
            </>
          )}
        </>
      )}

      <div className="saved-hint">Changes save automatically · close to keep it running in the tray</div>
    </div>
  )
}
