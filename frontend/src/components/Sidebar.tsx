import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  Server,
  Users,
  BarChart2,
  Terminal,
  Package,
  Settings,
  Zap,
} from 'lucide-react'
import { clsx } from 'clsx'

const NAV = [
  { to: '/',          icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/servers',   icon: Server,          label: 'Servers' },
  { to: '/players',   icon: Users,           label: 'Players' },
  { to: '/analytics', icon: BarChart2,       label: 'Analytics' },
  { to: '/mods',      icon: Package,         label: 'Mod Browser' },
  { to: '/console',   icon: Terminal,        label: 'Console' },
]

export function Sidebar() {
  return (
    <aside className="flex flex-col w-60 h-screen bg-dark-card border-r border-dark-border shrink-0">
      <div className="flex items-center gap-2.5 px-5 py-5 border-b border-dark-border">
        <div className="w-8 h-8 rounded-lg bg-nova-600 flex items-center justify-center">
          <Zap className="w-4 h-4 text-white" />
        </div>
        <div>
          <p className="font-bold text-sm leading-tight">NovaBox</p>
          <p className="text-[10px] text-dark-400 leading-tight">Minecraft Host</p>
        </div>
      </div>

      <nav className="flex-1 px-3 py-4 space-y-1 overflow-y-auto">
        {NAV.map(({ to, icon: Icon, label }) => (
          <NavLink
            key={to}
            to={to}
            end={to === '/'}
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors',
                isActive
                  ? 'bg-nova-600/15 text-nova-400 font-medium'
                  : 'text-dark-400 hover:text-white hover:bg-dark-border'
              )
            }
          >
            <Icon className="w-4 h-4 shrink-0" />
            {label}
          </NavLink>
        ))}
      </nav>

      <div className="px-3 pb-4 border-t border-dark-border pt-4">
        <NavLink
          to="/settings"
          className={({ isActive }) =>
            clsx(
              'flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors',
              isActive
                ? 'bg-nova-600/15 text-nova-400 font-medium'
                : 'text-dark-400 hover:text-white hover:bg-dark-border'
            )
          }
        >
          <Settings className="w-4 h-4 shrink-0" />
          Settings
        </NavLink>
        <p className="text-[10px] text-dark-600 px-3 mt-3">v0.1.0 · Free & Unlocked</p>
      </div>
    </aside>
  )
}
