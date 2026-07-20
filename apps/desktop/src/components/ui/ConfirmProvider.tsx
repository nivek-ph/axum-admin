import * as AlertDialog from '@radix-ui/react-alert-dialog'
import { createContext, useContext, useRef, useState, type ReactNode } from 'react'

import { Button } from './Button'

type Confirm = (message: string) => Promise<boolean>

const ConfirmContext = createContext<Confirm | null>(null)

export function ConfirmProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState('')
  const [open, setOpen] = useState(false)
  const resolveRef = useRef<((confirmed: boolean) => void) | null>(null)

  function finish(confirmed: boolean) {
    resolveRef.current?.(confirmed)
    resolveRef.current = null
    setOpen(false)
  }

  const confirm: Confirm = (nextMessage) => {
    resolveRef.current?.(false)
    setMessage(nextMessage)
    setOpen(true)
    return new Promise((resolve) => {
      resolveRef.current = resolve
    })
  }

  return (
    <ConfirmContext.Provider value={confirm}>
      {children}
      <AlertDialog.Root
        open={open}
        onOpenChange={(nextOpen) => {
          if (!nextOpen) finish(false)
        }}
      >
        <AlertDialog.Portal>
          <AlertDialog.Overlay className="modal-overlay" />
          <AlertDialog.Content className="confirm-content">
            <AlertDialog.Title>Confirm action</AlertDialog.Title>
            <AlertDialog.Description>{message}</AlertDialog.Description>
            <div className="modal-footer">
              <AlertDialog.Cancel asChild>
                <Button onClick={() => finish(false)}>Cancel</Button>
              </AlertDialog.Cancel>
              <AlertDialog.Action asChild>
                <Button onClick={() => finish(true)} variant="danger">
                  Confirm
                </Button>
              </AlertDialog.Action>
            </div>
          </AlertDialog.Content>
        </AlertDialog.Portal>
      </AlertDialog.Root>
    </ConfirmContext.Provider>
  )
}

export function useConfirm() {
  const confirm = useContext(ConfirmContext)
  if (!confirm) throw new Error('useConfirm must be used within ConfirmProvider')
  return confirm
}
