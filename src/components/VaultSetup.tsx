import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen } from "lucide-react";

import { Button } from "@/components/ui/button";
import { commands } from "@/bindings";

/**
 * 首次启动 vault 配置页面 —— 用户未配置 Obsidian vault 时显示。
 *
 * - 成功选择并校验通过 → 调 onConfigured,父组件切换到正常 App UI;
 * - 用户点取消并退出 → 调 Rust 侧 `exit_app` 命令,触发 Tauri 正常退出。
 */
interface VaultSetupProps {
  onConfigured: () => void;
}

export function VaultSetup({ onConfigured }: VaultSetupProps) {
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);

  async function handleSelect() {
    setError(null);
    setLoading(true);
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "选择 Obsidian Vault 根目录",
      });
      // 用户在原生对话框里点取消,不算错误,回到空闲态等再次选择
      if (selected === null) {
        setLoading(false);
        return;
      }

      const result = await commands.setVaultPath(selected as string);
      if (result.status === "error") {
        const err = result.error;
        if (err.kind === "Config") {
          setError(err.message);
        } else if (err.kind === "VaultNotConfigured") {
          setError("内部错误:vault 配置状态异常");
        } else {
          setError(`配置失败: ${JSON.stringify(err)}`);
        }
        setLoading(false);
        return;
      }

      onConfigured();
    } catch (e) {
      setError(`打开目录选择失败: ${String(e)}`);
      setLoading(false);
    }
  }

  async function handleExit() {
    await commands.exitApp();
  }

  return (
    <div className="flex min-h-screen items-center justify-center bg-background p-6">
      <div className="w-full max-w-md space-y-6">
        <div className="space-y-2">
          <h1 className="text-2xl font-semibold">初始化 KeySight</h1>
          <p className="text-sm text-muted-foreground">
            请选择你的 Obsidian vault 根目录。所选目录必须包含{" "}
            <code className="rounded bg-muted px-1 py-0.5 text-xs">.obsidian/</code>{" "}
            子目录。
          </p>
        </div>

        {error && (
          <div
            role="alert"
            className="rounded-md border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive"
          >
            {error}
          </div>
        )}

        <div className="flex flex-col gap-2">
          <Button onClick={handleSelect} disabled={loading} size="lg">
            <FolderOpen />
            {loading ? "配置中..." : "选择 Vault 目录"}
          </Button>

          <Button
            onClick={handleExit}
            variant="ghost"
            size="lg"
            disabled={loading}
          >
            取消并退出
          </Button>
        </div>
      </div>
    </div>
  );
}
