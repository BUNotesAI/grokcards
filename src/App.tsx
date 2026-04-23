import { useEffect, useState } from "react";
import { BrowserRouter, Routes, Route, Navigate } from "react-router-dom";
import AppShell from "@/components/AppShell";
import { KeysightView } from "@/components/keysight/KeysightView";
import { KanbanView } from "@/components/kanban/KanbanView";
import { VaultSetup } from "@/components/VaultSetup";
import TodoPage from "@/pages/TodoPage";
import PlaceholderPage from "@/pages/PlaceholderPage";
import { commands } from "@/bindings";
import { FileText, Bookmark, Code2, Settings } from "lucide-react";

// 启动时的 vault 配置探测状态:
// - loading: 正在读 get_vault_config;
// - needs_setup: runtime 未 ready(未配置 / bootstrap 失败)→ 展示 VaultSetup;
// - ready: runtime 已 install 可用 → 正常渲染 AppShell + 路由。
//
// 以 runtime `ready` 而不是 `vaultPath` 作为 gate:debug env override 场景
// (config.json 不存在但 runtime 已 bootstrap)、startup bootstrap 失败场景
// (config.json 存在但 runtime 未 install)都能正确路由。
type BootState = "loading" | "needs_setup" | "ready";

function App() {
  const [bootState, setBootState] = useState<BootState>("loading");

  useEffect(() => {
    let cancelled = false;
    (async () => {
      const result = await commands.getVaultConfig();
      if (cancelled) return;
      if (result.status === "ok" && result.data.ready) {
        setBootState("ready");
      } else {
        setBootState("needs_setup");
      }
    })();
    return () => {
      cancelled = true;
    };
  }, []);

  if (bootState === "loading") {
    return (
      <div className="flex min-h-screen items-center justify-center bg-background text-sm text-muted-foreground">
        Loading...
      </div>
    );
  }

  if (bootState === "needs_setup") {
    return <VaultSetup onConfigured={() => setBootState("ready")} />;
  }

  return (
    <BrowserRouter>
      <Routes>
        <Route element={<AppShell />}>
          <Route path="/keysight" element={<KeysightView />} />
          <Route path="/kanban" element={<KanbanView />} />
          <Route path="/todo" element={<TodoPage />} />
          <Route
            path="/notes"
            element={
              <PlaceholderPage
                icon={FileText}
                title="Notes"
                description="Quick notes and scratch pad. Markdown support, local-first storage."
              />
            }
          />
          <Route
            path="/bookmarks"
            element={
              <PlaceholderPage
                icon={Bookmark}
                title="Bookmarks"
                description="Save and organize links. Tag-based filtering, full-text search."
              />
            }
          />
          <Route
            path="/snippets"
            element={
              <PlaceholderPage
                icon={Code2}
                title="Snippets"
                description="Code snippets library. Syntax highlighting, quick copy, language tags."
              />
            }
          />
          <Route
            path="/settings"
            element={
              <PlaceholderPage
                icon={Settings}
                title="Settings"
                description="App preferences, theme, data management, keyboard shortcuts."
              />
            }
          />
          <Route path="*" element={<Navigate to="/keysight" replace />} />
        </Route>
      </Routes>
    </BrowserRouter>
  );
}

export default App;
