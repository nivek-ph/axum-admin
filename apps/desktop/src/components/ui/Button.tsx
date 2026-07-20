import type { ButtonHTMLAttributes } from 'react'

import { cn } from '@/lib/utils'

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'ghost' | 'danger'
}

export function Button({ className, variant = 'secondary', ...props }: ButtonProps) {
  return <button className={cn('button', `button-${variant}`, className)} {...props} />
}
