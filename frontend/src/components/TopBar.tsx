import { NavLink, useNavigate } from 'react-router-dom'
import { Zap, Settings, Plus, LogOut } from 'lucide-react'
import { clsx } from 'clsx'
import { useServers } from '@/hooks/useServers'
import { useAuth } from '@/context/AuthContext'

export function TopBar() {
  const { servers } = useServers({ poll: true, includeLiveCounts: false })
  const { user, isAdmin, logout, can } = useAuth()
  const navigate = useNavigate()

  return (
    <header className="flex items-stretch h-11 bg-dark-card border-b border-dark-border shrink-0 min-w-0">
      <div className="flex items-center gap-2 px-4 border-r border-dark-border shrink-0">
        <div className="w-6 h-6 rounded-md bg-nova-600 flex items-center justify-center">
          <Zap className="w-3.5 h-3.5 text-white" />
        </div>
        <span className="font-bold text-sm hidden sm:block">NovaBox</span>
      </div>

      <nav className="flex items-stretch overflow-x-auto scrollbar-none flex-1 min-w-0">
        <NavLink
          to="/"
          end
          className={({ isActive }) =>
            clsx(
              'flex items-center px-4 text-sm border-b-2 transition-colors whitespace-nowrap shrink-0',
              isActive
                ? 'border-nova-500 text-white font-medium bg-dark-800/30'
                : 'border-transparent text-dark-400 hover:text-white hover:bg-dark-800/20'
            )
          }
        >
          Dashboard
        </NavLink>

        {servers.map((s) => (
          <NavLink
            key={s.id}
            to={`/servers/${s.id}`}
            className={({ isActive }) =>
              clsx(
                'flex items-center gap-2 px-4 text-sm border-b-2 transition-colors whitespace-nowrap shrink-0',
                isActive
                  ? 'border-nova-500 text-white font-medium bg-dark-800/30'
                  : 'border-transparent text-dark-400 hover:text-white hover:bg-dark-800/20'
              )
            }
          >
            <span
              className={clsx(
                'w-1.5 h-1.5 rounded-full shrink-0',
                s.status === 'running'
                  ? 'bg-emerald-400'
                  : s.status === 'starting' || s.status === 'stopping'
                  ? 'bg-yellow-400 animate-pulse'
                  : 'bg-dark-500'
              )}
            />
            {s.name}
          </NavLink>
        ))}

        {can('servers.create') && (
          <button
            onClick={() => navigate('/servers/new')}
            title="Add server"
            className="flex items-center justify-center px-3 border-b-2 border-transparent text-dark-500 hover:text-dark-300 transition-colors shrink-0"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        )}
      </nav>

      <div className="flex items-stretch border-l border-dark-border shrink-0">
        {isAdmin && (
          <NavLink
            to="/users"
            className={({ isActive }) =>
              clsx(
                'flex items-center px-3 text-sm border-b-2 transition-colors',
                isActive
                  ? 'border-nova-500 text-white font-medium'
                  : 'border-transparent text-dark-400 hover:text-white'
              )
            }
          >
            Users
          </NavLink>
        )}

        {isAdmin && (
          <NavLink
            to="/settings"
            className={({ isActive }) =>
              clsx(
                'flex items-center px-3 border-b-2 transition-colors',
                isActive
                  ? 'border-nova-500 text-nova-400'
                  : 'border-transparent text-dark-400 hover:text-white'
              )
            }
          >
            <Settings className="w-4 h-4" />
          </NavLink>
        )}

        {user && (
          <div className="flex items-center gap-2 px-3 border-l border-dark-border">
            <div
              className={clsx(
                'w-6 h-6 rounded-full flex items-center justify-center text-[10px] font-bold shrink-0',
                isAdmin ? 'bg-nova-600/20 text-nova-400' : 'bg-dark-700 text-dark-300'
              )}
              title={isAdmin ? 'Admin' : 'User'}
            >
              {user.username[0]?.toUpperCase()}
            </div>
            <span className="text-xs text-dark-300 hidden md:block max-w-[100px] truncate">
              {user.username}
            </span>
            <button
              onClick={logout}
              title="Sign out"
              className="p-1 text-dark-500 hover:text-red-400 transition-colors"
            >
              <LogOut className="w-3.5 h-3.5" />
            </button>
          </div>
        )}
      </div>
    </header>
  )
}
