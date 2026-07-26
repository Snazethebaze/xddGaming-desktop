export interface Match {
  want_id: string
  who: string
  owner_id: string
  note: string | null
  priority: number
  priorityLabel: string
}

// Emitted by Rust on the "lookup" event.
export interface LookupPayload {
  state: 'pending' | 'result' | 'empty' | 'error'
  item: string
  me: string // my identity_id — hide "Found" on my own rows
  matches: Match[]
  message: string
}

export interface Settings {
  token: string
  hotkey: string
  identity_id: string
  identity_name: string
  toast_enabled: boolean
  toast_sound: boolean
  toast_corner: string
  poll_secs: number
}

export interface Person {
  id: string
  display_name: string
}

export interface PeopleResult {
  league: string
  people: Person[]
}
