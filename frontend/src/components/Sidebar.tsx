import { NavLink } from 'react-router-dom'
import {
  LayoutDashboard,
  Server,
  BarChart2,
  Package,
  Settings,
  Zap,
  Users,
  ShieldCheck,
  LogOut,
  User,
} from 'lucide-react'
import { clsx } from 'clsx'
import { useAuth } from '@/context/AuthContext'

const NAV = [
  { to: '/',          icon: LayoutDashboard, label: 'Dashboard' },
  { to: '/servers',   icon: Server,          label: 'Servers' },
  { to: '/analytics', icon: BarChart2,       label: 'Analytics' },
  { to: '/mods',      icon: Package,         label: 'Mod Browser' },
]

export function Sidebar() {
  const { user, isAdmin, logout } = useAuth()

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

        {isAdmin && (
          <NavLink
            to="/users"
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors',
                isActive
                  ? 'bg-nova-600/15 text-nova-400 font-medium'
                  : 'text-dark-400 hover:text-white hover:bg-dark-border'
              )
            }
          >
            <Users className="w-4 h-4 shrink-0" />
            Users
          </NavLink>
        )}
      </nav>

      <div className="px-3 pb-4 border-t border-dark-border pt-4 space-y-1">
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

        {user && (
          <div className="flex items-center gap-2 px-3 py-2 mt-1">
            <div className={clsx(
              'w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0',
              isAdmin ? 'bg-nova-600/20 text-nova-400' : 'bg-dark-700 text-dark-300'
            )}>
              {user.username[0]?.toUpperCase()}
            </div>
            <div className="flex-1 min-w-0">
              <p className="text-xs font-medium truncate leading-tight">{user.username}</p>
              <p className="text-[10px] text-dark-500 leading-tight flex items-center gap-1">
                {isAdmin
                  ? <><ShieldCheck className="w-2.5 h-2.5 inline" /> Admin</>
                  : <><User className="w-2.5 h-2.5 inline" /> User</>}
              </p>
            </div>
            <button
              onClick={logout}
              title="Sign out"
              className="p-1 text-dark-500 hover:text-red-400 transition-colors shrink-0"
            >
              <LogOut className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        <p className="text-[10px] text-dark-600 px-3 mt-1">v0.1.0 · Free & Unlocked</p>
      </div>
    </aside>
  )
}
