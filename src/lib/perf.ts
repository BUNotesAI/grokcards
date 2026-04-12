/**
 * 启动性能日志 + 冻结/卡顿监控 — 给"app 偶尔卡几分钟"调查用。
 *
 * 三层观测：
 * 1. perfLog — 手动埋点（main / GraphView mount / queries / first paint）
 * 2. heartbeat — 每 500ms 自检，主线程被阻塞 > 800ms 时记录长卡顿
 * 3. PerformanceObserver longtask — 任何 > 50ms 的 JS 任务自动记录
 *
 * 持久化：所有事件写 localStorage["keysight:perf:events"]，最多 500 条 ring buffer。
 * **冻结期间 console.log 看不到**，但 localStorage 写入会在冻结结束后保留下来。
 * 下次启动时 perf.ts 加载会自动 dump 上一 session 的最后 500 条事件，标记 [prev]。
 *
 * 重置：调 `clearPerfLog()`。手动 dump：`dumpPerfLog()`。
 */

const APP_START = performance.now();
const PERSIST_KEY = "keysight:perf:events";
const MAX_EVENTS = 500;
const events: string[] = [];

function nowTs(): string {
  return new Date().toISOString().slice(11, 23); // HH:mm:ss.sss
}

function persist(line: string): void {
  events.push(line);
  if (events.length > MAX_EVENTS) events.shift();
  try {
    localStorage.setItem(PERSIST_KEY, JSON.stringify(events));
  } catch {
    // localStorage 满或不可用时，丢日志而不是崩
  }
}

/** 上次 session 持久化的事件，模块加载时一次性回放到 console */
function dumpPreviousSession(): void {
  try {
    const raw = localStorage.getItem(PERSIST_KEY);
    if (!raw) return;
    const prev = JSON.parse(raw) as string[];
    if (!Array.isArray(prev) || prev.length === 0) return;
    // eslint-disable-next-line no-console
    console.log(
      `[perf prev session] === 上次启动遗留 ${prev.length} 条事件 ===`,
    );
    for (const line of prev) {
      // eslint-disable-next-line no-console
      console.log(`[perf prev] ${line}`);
    }
    // eslint-disable-next-line no-console
    console.log(`[perf prev session] === end ===`);
  } catch {
    // ignore
  }
}

dumpPreviousSession();

export function perfLog(message: string): void {
  const now = performance.now();
  const elapsed = now - APP_START;
  const line = `${nowTs()} +${elapsed.toFixed(0)}ms ${message}`;
  // eslint-disable-next-line no-console
  console.log(`[perf ${line}]`);
  persist(line);
}

/** 手动 dump 所有 in-memory 事件到 console（DevTools 里调用） */
export function dumpPerfLog(): void {
  // eslint-disable-next-line no-console
  console.log(`[perf dump] === ${events.length} events ===`);
  for (const line of events) {
    // eslint-disable-next-line no-console
    console.log(`[perf] ${line}`);
  }
}

/** 清空持久化的事件 */
export function clearPerfLog(): void {
  events.length = 0;
  try {
    localStorage.removeItem(PERSIST_KEY);
  } catch {
    // ignore
  }
}

// 暴露到 window 以便 DevTools console 调试
if (typeof window !== "undefined") {
  (window as unknown as { __perf: { dump: () => void; clear: () => void } }).__perf = {
    dump: dumpPerfLog,
    clear: clearPerfLog,
  };
}

/** 心跳检测：setInterval 每 500ms，比对实际时间和理论时间差。
 *  如果实际间隔 > 800ms，说明主线程被阻塞了至少 300ms。
 *  调用 startHeartbeat() 后才开始；只能启动一次。 */
let heartbeatStarted = false;
let lastHeartbeatTick = 0;
const HEARTBEAT_MS = 500;
const HEARTBEAT_WARN_MS = 800;

export function startHeartbeat(): void {
  if (heartbeatStarted) return;
  heartbeatStarted = true;
  lastHeartbeatTick = performance.now();
  perfLog(`heartbeat started (every ${HEARTBEAT_MS}ms, warn > ${HEARTBEAT_WARN_MS}ms)`);
  setInterval(() => {
    const now = performance.now();
    const actual = now - lastHeartbeatTick;
    lastHeartbeatTick = now;
    if (actual > HEARTBEAT_WARN_MS) {
      perfLog(
        `⚠️ HEARTBEAT GAP: 主线程阻塞 ${(actual - HEARTBEAT_MS).toFixed(0)}ms (实际间隔 ${actual.toFixed(0)}ms)`,
      );
    }
  }, HEARTBEAT_MS);
}

/** PerformanceObserver longtask — 浏览器自动通知 > 50ms 的 JS 任务 */
let longtaskStarted = false;
export function startLongTaskObserver(): void {
  if (longtaskStarted) return;
  longtaskStarted = true;
  if (typeof PerformanceObserver === "undefined") {
    perfLog("longtask observer 不支持 (no PerformanceObserver)");
    return;
  }
  try {
    const observer = new PerformanceObserver((list) => {
      for (const entry of list.getEntries()) {
        if (entry.duration < 100) continue; // 100ms 以下不记，否则太吵
        perfLog(
          `🐢 LONGTASK ${entry.duration.toFixed(0)}ms @ ${entry.startTime.toFixed(0)} (${entry.name})`,
        );
      }
    });
    observer.observe({ entryTypes: ["longtask"] });
    perfLog("longtask observer 启动 (warn > 100ms)");
  } catch (e) {
    perfLog(`longtask observer 启动失败: ${(e as Error).message}`);
  }
}

/** 全局错误捕获 — unhandled rejection / window error 都记下来 */
let errorCaptureStarted = false;
export function startErrorCapture(): void {
  if (errorCaptureStarted) return;
  errorCaptureStarted = true;
  if (typeof window === "undefined") return;
  window.addEventListener("error", (e) => {
    perfLog(`❌ ERROR: ${e.message} @ ${e.filename}:${e.lineno}`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    perfLog(`❌ UNHANDLED REJECTION: ${String(e.reason)}`);
  });
  perfLog("error capture 启动");
}

/** 包装一段代码计算耗时，返回结果 */
export function perfMeasure<T>(name: string, fn: () => T): T {
  const start = performance.now();
  const result = fn();
  const dur = performance.now() - start;
  // eslint-disable-next-line no-console
  console.log(`[perf ${dur.toFixed(0)}ms] ${name}`);
  return result;
}

/** 包装异步代码计算耗时 */
export async function perfMeasureAsync<T>(
  name: string,
  fn: () => Promise<T>,
): Promise<T> {
  const start = performance.now();
  try {
    return await fn();
  } finally {
    const dur = performance.now() - start;
    // eslint-disable-next-line no-console
    console.log(`[perf ${dur.toFixed(0)}ms] ${name}`);
  }
}
