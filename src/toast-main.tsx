import ReactDOM from 'react-dom/client'
import Toast from './Toast'
import './theme.css'

document.body.classList.add('toast-body')
ReactDOM.createRoot(document.getElementById('root')!).render(<Toast />)
