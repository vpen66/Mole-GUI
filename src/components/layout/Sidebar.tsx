import { NavLink } from "react-router-dom";
import { useT } from "@/i18n";
import {
  Trash2,
  Download,
  FolderOpen,
  Zap,
  History,
  LayoutDashboard,
  HardDrive,
  Settings2,
} from "lucide-react";

const navItems = [
  { to: "/", icon: LayoutDashboard, labelKey: "nav.dashboard" },
  { to: "/clean", icon: Trash2, labelKey: "nav.clean" },
  { to: "/uninstall", icon: Download, labelKey: "nav.uninstall" },
  { to: "/purge", icon: FolderOpen, labelKey: "nav.purge" },
  { to: "/optimize", icon: Zap, labelKey: "nav.optimize" },
  { to: "/analyze", icon: HardDrive, labelKey: "nav.analyze" },
  { to: "/history", icon: History, labelKey: "nav.history" },
  { to: "/settings", icon: Settings2, labelKey: "nav.settings" },
];

export function Sidebar() {
  const { t } = useT();

  return (
    <aside className="w-56 flex flex-col bg-surface-900 border-r border-surface-700 py-4 shrink-0">
      <div className="px-5 mb-8">
        <div className="flex items-center gap-2">
          <div className="w-8 h-8 bg-mole-600 rounded-lg flex items-center justify-center text-white font-bold text-sm">
            M
          </div>
          <div>
            <div className="font-semibold text-sm">Mole</div>
            <div className="text-xs text-surface-400">{t("nav.subtitle")}</div>
          </div>
        </div>
      </div>

      <nav className="flex flex-col gap-1 px-3">
        {navItems.map(({ to, icon: Icon, labelKey }) => (
          <NavLink
            key={to}
            to={to}
            end={to === "/"}
            className={({ isActive }) =>
              `flex items-center gap-3 px-3 py-2 rounded-lg text-sm transition-colors ${
                isActive
                  ? "bg-mole-600/20 text-mole-400 font-medium"
                  : "text-surface-300 hover:bg-surface-800 hover:text-surface-100"
              }`
            }
          >
            <Icon size={16} />
            {t(labelKey)}
          </NavLink>
        ))}
      </nav>

      <div className="mt-auto px-5 pt-4 border-t border-surface-700">
        <div className="text-xs text-surface-500">Mole GUI v1.0.0</div>
      </div>
    </aside>
  );
}
