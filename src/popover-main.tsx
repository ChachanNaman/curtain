import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import Popover from '@/routes/popover/Popover'
import '@/index.css'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Popover />
  </StrictMode>,
)
