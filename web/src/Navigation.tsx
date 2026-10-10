export type Page = 'verify' | 'pulse' | 'chart'

const LINKS: { page: Page; href: string; label: string }[] = [
  { page: 'verify', href: '/', label: 'Verify a price' },
  { page: 'pulse', href: '/pulse', label: 'Pulse vote' },
  { page: 'chart', href: '/chart', label: 'Chart' },
]

export default function Navigation({ current }: { current: Page }) {
  return (
    <nav className="site-nav" aria-label="Primary navigation">
      {LINKS.map((link) => (
        <a key={link.page} href={link.href} aria-current={link.page === current ? 'page' : undefined}>
          {link.label}
        </a>
      ))}
    </nav>
  )
}
