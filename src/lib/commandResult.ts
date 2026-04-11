/**
 * 解包 tauri-specta typedError 的返回值。
 *
 * bindings.ts 中的 commands 返回 { status: "ok", data: T } | { status: "error", error: E }。
 * 此函数提取 data 或将 error 转为 throw，适配 TanStack Query 的 queryFn 期望。
 */
export async function unwrapCommand<T, E>(
  promise: Promise<{ status: "ok"; data: T } | { status: "error"; error: E }>,
): Promise<T> {
  const result = await promise;
  if (result.status === "ok") {
    return result.data;
  }
  throw result.error;
}
