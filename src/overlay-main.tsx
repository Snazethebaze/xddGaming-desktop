import ReactDOM from 'react-dom/client'
import Overlay from './Overlay'
import './theme.css'

document.body.classList.add('overlay-body')
ReactDOM.createRoot(document.getElementById('root')!).render(<Overlay />)
