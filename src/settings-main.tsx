import ReactDOM from 'react-dom/client'
import Settings from './Settings'
import './theme.css'

document.body.classList.add('settings-body')
ReactDOM.createRoot(document.getElementById('root')!).render(<Settings />)
