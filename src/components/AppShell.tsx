import { NavLink, Outlet, useLocation } from "react-router-dom";
import {
  Sidebar,
  SidebarContent,
  SidebarFooter,
  SidebarGroup,
  SidebarGroupContent,
  SidebarGroupLabel,
  SidebarHeader,
  SidebarInset,
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  SidebarProvider,
  SidebarRail,
  SidebarTrigger,
  useSidebar,
} from "@/components/ui/sidebar";
import { TooltipProvider } from "@/components/ui/tooltip";
import {
  CheckSquare,
  FileText,
  Bookmark,
  Code2,
  Grid3x3,
  Settings,
  Zap,
} from "lucide-react";

const modules = [
  { label: "KeySight", icon: Grid3x3, path: "/keysight" },
  { label: "Todo", icon: CheckSquare, path: "/todo" },
  { label: "Notes", icon: FileText, path: "/notes" },
  { label: "Bookmarks", icon: Bookmark, path: "/bookmarks" },
  { label: "Snippets", icon: Code2, path: "/snippets" },
];

const system = [
  { label: "Settings", icon: Settings, path: "/settings" },
];

const pageTitles: Record<string, string> = {
  "/keysight": "KeySight",
  "/todo": "Todo",
  "/notes": "Notes",
  "/bookmarks": "Bookmarks",
  "/snippets": "Snippets",
  "/settings": "Settings",
};

function AppSidebar() {
  const { state } = useSidebar();
  const collapsed = state === "collapsed";
  const location = useLocation();

  return (
    <Sidebar collapsible="icon">
      <SidebarHeader className="border-b border-sidebar-border">
        <div className="flex h-8 items-center gap-2.5 px-1">
          {collapsed ? (
            <SidebarTrigger className="size-6" />
          ) : (
            <>
              <div className="flex size-6 shrink-0 items-center justify-center rounded-md bg-foreground">
                <Zap className="size-3.5 text-background" />
              </div>
              <span className="flex-1 font-heading text-[15px] tracking-tight">
                Super Tauri
              </span>
              <SidebarTrigger className="size-6 text-muted-foreground" />
            </>
          )}
        </div>
      </SidebarHeader>

      <SidebarContent>
        <SidebarGroup>
          <SidebarGroupLabel>Modules</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {modules.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <SidebarMenuButton
                    tooltip={item.label}
                    isActive={location.pathname === item.path}
                    render={<NavLink to={item.path} />}
                  >
                    <item.icon className="size-4" />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>

        <SidebarGroup>
          <SidebarGroupLabel>System</SidebarGroupLabel>
          <SidebarGroupContent>
            <SidebarMenu>
              {system.map((item) => (
                <SidebarMenuItem key={item.path}>
                  <SidebarMenuButton
                    tooltip={item.label}
                    isActive={location.pathname === item.path}
                    render={<NavLink to={item.path} />}
                  >
                    <item.icon className="size-4" />
                    <span>{item.label}</span>
                  </SidebarMenuButton>
                </SidebarMenuItem>
              ))}
            </SidebarMenu>
          </SidebarGroupContent>
        </SidebarGroup>
      </SidebarContent>

      <SidebarFooter className="border-t border-sidebar-border">
        <div className="flex items-center gap-2.5 px-1">
          <div className="flex size-7 shrink-0 items-center justify-center rounded-full bg-gradient-to-br from-accent to-yellow-600 text-xs font-bold text-white">
            A
          </div>
          {!collapsed && (
            <div className="min-w-0 flex-1">
              <div className="text-xs font-medium">Alex</div>
              <div className="text-[11px] text-muted-foreground">Local</div>
            </div>
          )}
        </div>
      </SidebarFooter>

      <SidebarRail />
    </Sidebar>
  );
}

function Topbar() {
  const location = useLocation();
  const title = pageTitles[location.pathname] ?? "";

  return (
    <header className="flex h-[52px] shrink-0 items-center gap-3 border-b bg-card px-4">
      <h1 className="font-heading text-[17px]">{title}</h1>
    </header>
  );
}

/**
 * 内容区域容器
 *
 * GraphView 需要全屏（无 padding、无滚动），其他页面保留常规布局。
 * 根据当前路径动态切换样式。
 */
function ContentArea() {
  const location = useLocation();
  // GraphView 需要全屏，不带 padding 和滚动
  const isFullscreen = location.pathname === "/keysight";

  return (
    <>
      <Topbar />
      <main
        className={
          isFullscreen
            ? "relative flex-1 overflow-hidden"
            : "flex-1 overflow-y-auto p-6"
        }
      >
        <Outlet />
      </main>
    </>
  );
}

export default function AppShell() {
  return (
    <TooltipProvider>
      <SidebarProvider defaultOpen={false}>
        <AppSidebar />
        <SidebarInset>
          <ContentArea />
        </SidebarInset>
      </SidebarProvider>
    </TooltipProvider>
  );
}
