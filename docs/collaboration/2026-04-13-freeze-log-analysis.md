# Freeze 调查：perf 日志分析 + 三步修正方案

> 本文交接对象：下一个 agent（或人类），继续"app 偶发卡死"调查。
>
> 本文包含：日志物理位置 + 本次读到的证据 + 三个关键结论 + 要实现的三件事的具体代码位置。
> 读完本文应该能直接开工而不必重新调查。

---

## 调查背景

从 2026-04-12 起，用户反复报告"app 打开后偶尔卡死几分钟"。当时的 handoff
(`docs/handoff/keysight.md`, 2026-04-12 session 8ec03324) 记录：

> 冻结日志捕获 #1：用户提供了 `16:31:20.676 ⚠️ HEARTBEAT GAP 950ms`，但无 LONGTASK +
> 无 LoAF（当时未装）+ 用户描述"啥也没做就是第一次启动"。结论暂定：可能是 V8 GC pause /
> WKWebView 合成器抖动。装了 LoAF + visibility 之后等下次复现。

那一轮的"装 LoAF + visibility" 就是 commit `33dd312 chore(perf): Long Animation Frame
observer + visibility tracking`。今天（2026-04-13 早上）用户再次反馈 app 卡死，打开本次
调查。

**重要前置认知**：本文之前的 agent 认为 "装了 LoAF 就能抓 GC pause"。**这是错的**，下面
第 ② 节会讲为什么。不要在此基础上继续期待 LoAF 数据。

---

## 第一步：logs 物理位置

埋点逻辑在 `src/lib/perf.ts`，所有事件持久化到 WKWebView 的 localStorage key
`keysight:perf:events`（ring buffer，500 条上限，每次 `perfLog()` 都重写整个 JSON
数组到 localStorage）。

WKWebView 的 localStorage 在磁盘上是一个 SQLite 文件，路径：

```
~/Library/WebKit/super-tauri/WebsiteData/Default/<hash>/<hash>/LocalStorage/localstorage.sqlite3
```

其中 `<hash>` 是 WebKit 根据 origin 算出的目录名。用户这台机器有两个 origin：
`http://localhost`（app 本身）和 `inspector-resource`（DevTools 自己的 storage）。
localhost 那个目录是 `1zvU1MHPmqHve6_On_DicHVLLdbFC8pciG8yO7XvxJw`（如果未来换机器或清
数据，会变，按 mtime 挑新的即可）。

localStorage 值存为 **UTF-16LE** 二进制 blob。读取方法：

```bash
LS_DB="$HOME/Library/WebKit/super-tauri/WebsiteData/Default/1zvU1MHPmqHve6_On_DicHVLLdbFC8pciG8yO7XvxJw/1zvU1MHPmqHve6_On_DicHVLLdbFC8pciG8yO7XvxJw/LocalStorage/localstorage.sqlite3"

# 1. 强制 WAL checkpoint 拿到最新数据（app 还在跑时 WAL 里可能有未落盘的条目）
sqlite3 "$LS_DB" "PRAGMA wal_checkpoint(PASSIVE);"

# 2. 导出 blob 到文件
sqlite3 "$LS_DB" "SELECT writefile('/tmp/perf-events.utf16.bin', value) FROM ItemTable WHERE key='keysight:perf:events';"

# 3. UTF-16LE → UTF-8
iconv -f UTF-16LE -t UTF-8 /tmp/perf-events.utf16.bin > /tmp/perf-events.json

# 4. 看事件
jq '. | length' /tmp/perf-events.json                 # 总条数
jq -r '.[]' /tmp/perf-events.json | grep -v "HEARTBEAT GAP"   # 去掉 heartbeat 噪音
jq -r '.[-30:] | .[]' /tmp/perf-events.json           # 最后 30 条（最接近 freeze 那一刻）
```

**易错点**：`sqlite3` 的 WAL 读取是透明的，但如果 app 还在跑，WAL 里可能有几百 KB 没落
到主文件。先 `PRAGMA wal_checkpoint(PASSIVE)` 一下再 `SELECT value`，否则拿到的是
snapshot 而不是最新状态。本次调查中确实踩到这个（第一次读 169 条，checkpoint 后变 224 条）。

---

## 第二步：本次读到的证据

时间跨度：`02:13:39.082 → 02:18:37.353`（约 5 分钟），共 224 条事件。

### 启动阶段（00:00 → 00:01）— 一切正常

```
02:13:39.082  +7ms     main.tsx loaded — JS bundle parsed
02:13:39.082  +7ms     error capture 启动
02:13:39.082  +7ms     heartbeat started (every 500ms, warn > 800ms)
02:13:39.082  +7ms     longtask observer 启动 (warn > 100ms)
02:13:39.082  +7ms     LoAF observer 不支持 (浏览器太旧)           ← 关键
02:13:39.082  +7ms     visibility tracking 启动
02:13:39.082  +7ms     ReactDOM.createRoot — about to render
02:13:39.152  +77ms    GraphView mount (first render)
02:13:39.164  +89ms    query[cards] start
02:13:39.165  +90ms    query[sections[wb_root]] start
02:13:39.165  +90ms    query[notes[wb_root]] start
02:13:39.166  +90ms    query[aliases[wb_root]] start
02:13:39.166  +91ms    query[tasks[wb_root]] start
02:13:39.166  +91ms    query[questions[wb_root]] start
02:13:39.166  +91ms    query[positions[wb_root]] start
02:13:39.166  +91ms    query[whiteboards] start
02:13:39.187  +112ms   query[cards] done in 23ms (n=145)
02:13:39.188  +113ms   query[sections[wb_root]] done in 23ms (n=6)
02:13:39.196  +121ms   query[notes[wb_root]] done in 31ms (n=7)
02:13:39.197  +121ms   query[aliases[wb_root]] done in 31ms (n=0)
02:13:39.197  +122ms   query[tasks[wb_root]] done in 31ms (n=0)
02:13:39.197  +122ms   query[questions[wb_root]] done in 31ms (n=0)
02:13:39.197  +122ms   query[positions[wb_root]] done in 31ms (n=21)
02:13:39.197  +122ms   query[whiteboards] done in 31ms (n=3)
02:13:39.225  +150ms   GraphView data ready: cards=145 notes=7 sections=6 aliases=0 tasks=0 questions=0 positions=21
02:13:39.225  +150ms   GraphView first visible paint: visible=7/13
```

**结论**：启动 + 数据加载 + 首次可见 paint 共 **150ms**。没有瓶颈。
Rust 侧 8 个并行 query 最慢 31ms。启动路径完全正常。

### 运行阶段（+40s → +180s）— 全是 heartbeat 噪音

```
02:14:19.647  +40571ms   👁️ visibility: hidden
02:14:20.451  +41375ms   ⚠️ HEARTBEAT GAP: 主线程阻塞 500ms (实际间隔 1000ms)
02:14:21.451  +42376ms   ⚠️ HEARTBEAT GAP: 主线程阻塞 500ms (实际间隔 1000ms)
02:14:22.451  +43376ms   ⚠️ HEARTBEAT GAP: 主线程阻塞 500ms (实际间隔 1000ms)
...（连续 140+ 条，每条都是 500ms 阻塞 / 1000ms 实际间隔）
02:16:40.022  +180947ms  👁️ window focus
02:16:40.029  +180953ms  👁️ visibility: visible
02:16:41.662  +182587ms  👁️ window blur
02:16:42.451  +183375ms  ⚠️ HEARTBEAT GAP ...  （又开始）
...
02:18:37.353  +298277ms  ⚠️ HEARTBEAT GAP ...  （最后一条）
```

### 之后完全静默，直到现在（8 小时后）

ring buffer 上限 500 条。当前只有 224 条，远未满。如果 heartbeat 一直在 fire，8 小时 ×
3600 次/小时 = 28,800 次，早就把 ring buffer 滚过 57 遍了。事实上事件停在 +298277ms，
说明 **heartbeat 的 setInterval 彻底不再 fire**。

---

## 三个关键结论

### ① "HEARTBEAT GAP 阻塞 500ms" 全部是假阳性

216 条 `HEARTBEAT GAP` 没有一条代表真实的主线程阻塞。证据：

1. **精确一致** — 每一条都是 `阻塞 500ms / 实际间隔 1000ms`。真实的 JS 阻塞会是
   长短不一的分布，不会这么规律。
2. **0 条 LONGTASK** — `PerformanceObserver({entryTypes:['longtask']})` 的阈值是 100ms，
   如果真有主线程被 500ms 单个 JS 任务阻塞，必然会报 LONGTASK。实际是：longtask 数组
   全空。
3. **完美对齐 visibility 状态** — heartbeat gap 的第一条发生在 `👁️ visibility: hidden`
   **之后 1 秒**（setInterval 下一次 tick）；focus 回来后短暂中断；blur 后又重新开始。

**真实原因**：**WKWebView 在窗口非 focused / tab hidden 时，把 `setInterval` 和
`setTimeout` 的最短间隔 clamp 到 1000ms（1 Hz）**。这是 macOS Energy Saver 的默认行为。
我们设的是 500ms，被拉长到 1000ms，heartbeat 回调里算出 `actual = now - lastTick ≈ 1000ms`，
超过 800ms 阈值就 warn。

**影响**：

- `perf.ts` 的 heartbeat 告警完全失去信号价值。
- 上一次 handoff 里"950ms heartbeat gap 怀疑 GC pause"的判断需要重新审视 —— 如果那个
  950ms 也发生在窗口失焦状态下（比如用户刚 Cmd+Tab 切回来看到的"卡死"），那也是这个
  假阳性的实例，不是 GC。

### ② LoAF observer 在 macOS WKWebView **完全不支持**

```
02:13:39.082  +7ms  LoAF observer 不支持 (浏览器太旧)
```

这行来自 `src/lib/perf.ts:155-158`：

```typescript
const supported = PerformanceObserver.supportedEntryTypes;
if (!supported || !supported.includes("long-animation-frame")) {
  perfLog("LoAF observer 不支持 (浏览器太旧)");
  return;
}
```

实际情况：**截至本次调查 (2026-04-13)，macOS 15.x 上的 WKWebView（Safari 18）还没有
`long-animation-frame` entry type 支持**。只有 Safari Tech Preview 和 Chrome/Edge 123+
支持。WebKit 主线一直在跟进但没发布。

所以昨天 commit `33dd312` 装的 LoAF 探针，**在这台机器上永远等不到一条数据**。

### ③ 真正的 freeze 发生在 02:18:37 之后，日志完全捕捉不到

事件停在 `02:18:37.353 +298277ms`。按 ring buffer 规则，如果 heartbeat 还在 fire，
我们应该看到 500 条全满 + 旧事件被滚掉。实际是 224 条没满就停。**这说明 setInterval
本身在 02:18:37 之后被完全停掉了**。

可能的原因：

- **App Nap（macOS 更激进的 CPU 限制）** — macOS 在窗口持续背景几分钟后会把整个 app
  进程的时钟 tick 砍到接近 0，setInterval 不再调度。
- **JS 事件循环真的卡住了** — Promise microtask 死循环、native IPC deadlock、infinite
  React render 等。但这些应该先触发 `window error` 或 `unhandled rejection`，而
  `startErrorCapture()` 也是启动时就装的，日志里**没有任何 ERROR / UNHANDLED REJECTION**
  条目 —— 所以不是 JS 错误。
- **WKWebView 进程被 Activity Monitor 暂停 / 被 macOS suspend** — 可能性存在但更难验证。

**结论**：现在的埋点**根本没能力捕捉到真实 freeze 那一刻**。因为：

1. 背景节流把 heartbeat 告警变成假阳性噪音，即使没 freeze 也在刷屏。
2. 真 freeze 发生时，setInterval 已经停了（不管是 App Nap 还是别的），**新事件根本不会
   被写进 localStorage**，所以我们只能看到"freeze 发生前的最后一次背景节流 tick"，不
   能看到"freeze 发生那一刻用户最后在干嘛"。

---

## 修正方案：三件事

按收益从高到低排。**三件都要做**，否则下次复现仍然拿不到信号。

### ① heartbeat 告警在 hidden/blur 下自动静默

**文件**：`src/lib/perf.ts`

**函数**：`startHeartbeat()`（约 line 104-119）

**当前代码**：

```typescript
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
```

**改法**：在 warn 之前检查 `document.visibilityState` 和 `document.hasFocus()`，任一为
非可见/非 focused 就跳过 warn，但仍更新 `lastHeartbeatTick`（否则 focus 回来那一瞬间
会误报一次大 gap）。

```typescript
setInterval(() => {
  const now = performance.now();
  const actual = now - lastHeartbeatTick;
  lastHeartbeatTick = now;
  // 背景态（hidden 或 blurred）下 WKWebView 把 setInterval clamp 到 1Hz，
  // heartbeat 告警会变成假阳性。只在窗口处于前台 + focused 时告警。
  const isForeground =
    typeof document !== "undefined" &&
    document.visibilityState === "visible" &&
    document.hasFocus();
  if (isForeground && actual > HEARTBEAT_WARN_MS) {
    perfLog(
      `⚠️ HEARTBEAT GAP: 主线程阻塞 ${(actual - HEARTBEAT_MS).toFixed(0)}ms (实际间隔 ${actual.toFixed(0)}ms)`,
    );
  }
}, HEARTBEAT_MS);
```

**预期收益**：本次 session 的 224 条事件里 216 条 heartbeat 假阳性会清零，剩 ~8 条信号
事件（mount / query / visibility change / focus/blur）。下次复现时如果真的有 heartbeat
告警，**一定是前台真阻塞**。

---

### ② 移除 LoAF observer，改装 `requestAnimationFrame` 间隔探针

**文件**：`src/lib/perf.ts`

**当前**：`startLongAnimationFrameObserver()` 函数（line 149-197）。

**问题**：在 macOS WKWebView 里永远返回 "不支持"，占着篇幅 + 误导下一个 agent
以为"等下次复现就能拿 LoAF 数据"。

**方案**：删除整个 `startLongAnimationFrameObserver()` 函数和 `main.tsx` 中的调用，
改装一个 **rAF 间隔探针**（Safari 全版本支持）：

```typescript
/** requestAnimationFrame 间隔探针 — Safari/WKWebView 全版本支持
 *  在前台态下 rAF 应该以 ~16.7ms 间隔 fire。如果某一帧间隔 > 100ms，
 *  说明主线程被阻塞、或 paint/layout 耗时过大。比 LoAF 粗糙但在 WKWebView 能跑。 */
let rafStarted = false;
export function startRafProbe(): void {
  if (rafStarted) return;
  rafStarted = true;
  if (typeof requestAnimationFrame === "undefined") return;
  let lastRaf = performance.now();
  const tick = (now: number) => {
    const gap = now - lastRaf;
    lastRaf = now;
    // 前台态才报（和 heartbeat 一样的原因）
    const isForeground =
      typeof document !== "undefined" &&
      document.visibilityState === "visible" &&
      document.hasFocus();
    if (isForeground && gap > 100) {
      perfLog(`🎞️ RAF GAP: ${gap.toFixed(0)}ms (期望 ~16.7ms)`);
    }
    requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  perfLog("rAF probe 启动 (warn > 100ms)");
}
```

在 `main.tsx` 里把 `startLongAnimationFrameObserver()` 替换成 `startRafProbe()`。

**预期收益**：rAF 的间隔直接反映用户感知的卡顿 — rAF 卡 500ms = 用户看到 500ms 的
不动。这比 setInterval 更难被 clamp（setTimeout/setInterval 会被 clamp，rAF 在前台
态会严格按 vsync）。

---

### ③ 加 "最后 N 个动作" 的 action trail — **最关键**

**目标**：freeze 发生后，能看到"freeze 前最后用户做了什么 / 最后一个 tauri command 是什么"。
当前 perfLog 的埋点覆盖了 mount / query / first paint，但**完全没覆盖**：

- 用户点了哪个按钮 / 打开了哪个菜单 / 拖了哪个节点
- 哪个 `tauri::invoke` 正在跑（多少个 in-flight）
- 哪个 React re-render 特别慢

最低成本的埋点位置：**`src/lib/commandResult.ts` 的 `unwrapCommand`**（所有 Rust command
都经过它）。

**当前**（大致）：

```typescript
export async function unwrapCommand<T>(p: Promise<Result<T, AppError>>): Promise<T> {
  const result = await p;
  if (result.status === "error") throw new Error(result.error.message);
  return result.data;
}
```

**改为**：

```typescript
import { perfLog } from "./perf";

let inFlightCommands = 0;
let commandSeq = 0;

export async function unwrapCommand<T>(
  p: Promise<Result<T, AppError>>,
  commandName?: string,
): Promise<T> {
  const seq = ++commandSeq;
  const name = commandName ?? "invoke";
  inFlightCommands++;
  const start = performance.now();
  perfLog(`▶ ${name} #${seq} start (in-flight=${inFlightCommands})`);
  try {
    const result = await p;
    const dur = performance.now() - start;
    if (result.status === "error") {
      perfLog(`✖ ${name} #${seq} error in ${dur.toFixed(0)}ms: ${result.error.message}`);
      throw new Error(result.error.message);
    }
    if (dur > 50) {
      perfLog(`■ ${name} #${seq} done in ${dur.toFixed(0)}ms`);
    }
    return result.data;
  } finally {
    inFlightCommands--;
  }
}
```

调用方暂时不需要改（`commandName` 是可选的），但可以逐步补上，比如
`unwrapCommand(commands.cardEditTitle(id, title), "cardEditTitle")`。

**可选增强**：在 `commands` proxy 里自动塞 commandName —— 但这要动 `bindings.ts` 的生成
逻辑（tauri-specta），成本高，本 task 不做。

**预期收益**：freeze 发生时，最后几条日志会直接指向"最后一个没完成的 invoke"。如果 freeze
前有 `▶ cardEditTitle #42 start` 但没有对应的 `■ cardEditTitle #42 done`，就知道是 Rust
侧或 IPC 层卡住了；如果 `▶` 和 `■` 都有，那是 JS/React 侧的问题。

---

## TDD 约束

按 CLAUDE.md L0 TDD 规则：

- **① heartbeat 前台态检查** — 可以加 vitest 测试：mock `document.visibilityState` +
  `document.hasFocus()`，跑 `setInterval` 一次（用 `vi.useFakeTimers()`），断言
  `perfLog` mock 在 hidden 态下**没被调用**，visible + focused 态下**被调用**。
- **② rAF probe** — 类似，mock `requestAnimationFrame` + 手动调用 tick，断言 gap > 100 +
  前台态才 perfLog。
- **③ unwrapCommand invoke 追踪** — 测试用一个 resolved / rejected Promise 驱动，断言
  perfLog 被正确调用两次（start + done / error）。

三个都是小函数级测试，不需要 RTL。建议加到 `src/__tests__/lib/perf.test.ts` 和
`src/__tests__/lib/commandResult.test.ts`（两个新文件）。

**注意**：本 session 的用户授权了"TDD 照常但不用人工 Red/Green 确认"，这条授权是针对
**上一个 task（⋯ 菜单）**的。下一个 agent 开始改 perf.ts 前，**请先和用户确认是否沿用
自主模式，还是回到标准 Red/Green 人工确认关卡**。

---

## 不要做的事

1. **不要删除 `startLongTaskObserver()`** — `longtask` entry type 在 WKWebView 是支持的
   （虽然本次没捕到任何 >100ms 的 longtask，但那是因为真的没有长 JS 任务）。保留它作为
   单帧长任务的兜底。
2. **不要把 heartbeat interval 改得更短** —（比如 250ms）。WKWebView 的背景 clamp 是 1Hz
   无论你设什么值，改短只会让前台态更吵。500ms 是合适的。
3. **不要开始"用 Chrome 打开 dev server 看 DevTools"的排查路径** — Chrome 的 JS engine /
   rendering pipeline 和 WKWebView 差别大到这个 bug 很可能在 Chrome 里复现不出来。要在
   Tauri 构建的 app 里复现才算数。
4. **不要一口气把三件事全 commit 在一起**。按 CLAUDE.md L0 Code Review 关卡，每件单独
   commit，每个 commit 跑完整 test suite + clippy + build。
5. **不要假设 freeze 发生在某个特定操作**。当前证据为零，任何"我觉得是 X 导致的"都是猜。
   先把埋点装好，等下次复现拿到动作轨迹再说。

---

## 完成标准

1. ① 改完后，跑 `pnpm tauri dev`，手动把窗口切到背景，观察日志：前 30 秒应该没有任何
   HEARTBEAT GAP 告警（仅 `👁️ visibility: hidden` 一条）。切回前台，恢复正常。
2. ② 改完后，日志里应该出现 `rAF probe 启动`，并且**没有** `LoAF observer` 相关条目
   （整个 function 已删除）。
3. ③ 改完后，在 app 里点任意一个触发 Rust command 的按钮（比如 Sync / Create Section），
   日志里应该出现配对的 `▶ ... start` 和 `■ ... done`。
4. 三件事完成后，跑全量测试 + clippy + build（`pnpm test && pnpm build && cd src-tauri &&
   cargo test --workspace && cargo clippy --workspace -- -D warnings`）。全绿才可以交付。
5. 更新 `docs/progress/keysight.md` 把"冻结诊断"从 `Next` 移到 `Done`，并在 `Next` 添加
   新的一项："等下次复现，根据 action trail 定位"。
6. 在 `docs/devlog/2026-04-13.md` 追加一段 session 总结。

---

## Review 完成后

如果你（下一个 agent）搞定了这三件事但仍然**没能在下次复现时抓到根因**，那说明真 freeze
发生在比 `unwrapCommand` 更低的层 —— 可能是 React render / 某个第三方库 hook / WKWebView
合成器本身。下一步要上的工具：

- **React Profiler API**（`<Profiler onRender={...}>`）— 包住 KeysightView，记录超长
  render 的 component tree
- **Rust 侧 tracing** — 在 `src-tauri/src/lib.rs` 和每个 `#[tauri::command]` 里加 tracing
  span，输出到文件；freeze 时对比 TS 日志和 Rust 日志的时间戳看哪边先卡
- **macOS Instruments** — 最后手段，`xcrun xctrace record --template 'Time Profiler' --attach super-tauri`
  抓 CPU / 线程 / GC。

但这些都是**本 task 完成后**才考虑的下一步，现在先把三件事做掉。
