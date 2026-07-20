import { cn } from '@/lib/utils'

interface BrandMarkProps {
  className?: string
  size?: 'default' | 'small'
}

export function BrandMark({ className, size = 'default' }: BrandMarkProps) {
  return (
    <div aria-hidden="true" className={cn('brand-mark', size === 'small' && 'small', className)}>
      <svg fill="none" viewBox="0 0 32 32">
        <rect fill="currentColor" height="22" opacity="0.28" rx="4" width="10" x="4" y="5" />
        <rect fill="currentColor" height="16" opacity="0.55" rx="4" width="10" x="11" y="8" />
        <rect fill="currentColor" height="22" rx="4" width="10" x="18" y="5" />
      </svg>
    </div>
  )
}
