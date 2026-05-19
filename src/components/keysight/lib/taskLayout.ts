import type { Position, TaskStatus } from "@/bindings";

export const TASK_NODE_WIDTH = 320;
export const TASK_NODE_HEIGHT = 140;
export const TASK_JUMP_MIN_ZOOM = 0.75;
export const TASK_TOP_INSERT_OFFSET_Y = 180;
export const TASK_PACK_COLUMN_GAP = 360;
export const TASK_PACK_ROW_GAP = 170;
export const TASK_PACK_TOP_OFFSET_Y = 200;

// Record<TaskStatus, ...> 守护:Rust 端 TaskStatus 加 variant 时 TS 编译
// 因缺 key 而失败,强制补 column。防 stringly typed 漏 case,符合 L0 防火墙
// "想出错都难"——比 array + indexOf 更直接表达"每个 status 必有一列"。
const TASK_STATUS_COLUMN: Record<TaskStatus, number> = {
  inbox: 0,
  next: 1,
  active: 2,
  blocked: 3,
  done: 4,
};
const TASK_STATUS_COLUMN_COUNT = Object.keys(TASK_STATUS_COLUMN).length;

/**
 * 返回比当前最顶端 task 再向上偏移 TASK_TOP_INSERT_OFFSET_Y 的位置;
 * 若一个 task 都没有则返回 fallback。多 task 同 y 时取 x 更小者。
 */
export function taskPositionAboveTopmost(
  tasks: Array<{ id: string }>,
  positions: Record<string, Position>,
  fallback: Position,
): Position {
  const taskPositions = tasks
    .map((task) => positions[task.id])
    .filter((position): position is Position => position != null);

  if (taskPositions.length === 0) return fallback;

  const topmost = taskPositions.reduce((best, position) => {
    if (position.y < best.y) return position;
    if (position.y === best.y && position.x < best.x) return position;
    return best;
  });

  return {
    x: topmost.x,
    y: topmost.y - TASK_TOP_INSERT_OFFSET_Y,
  };
}

/** 把 tasks 按 status 列、出现顺序行,以 viewportCenter 为锚整齐 pack 到画布 */
export function packTaskPositions(
  tasks: Array<{ id: string; status: TaskStatus }>,
  viewportCenter: Position,
): Record<string, Position> {
  const originX =
    viewportCenter.x - ((TASK_STATUS_COLUMN_COUNT - 1) * TASK_PACK_COLUMN_GAP) / 2;
  const originY = viewportCenter.y - TASK_PACK_TOP_OFFSET_Y;
  const rowsByColumn = new Map<number, number>();
  const packed: Record<string, Position> = {};

  for (const task of tasks) {
    const column = TASK_STATUS_COLUMN[task.status];
    const row = rowsByColumn.get(column) ?? 0;
    rowsByColumn.set(column, row + 1);
    packed[task.id] = {
      x: originX + column * TASK_PACK_COLUMN_GAP,
      y: originY + row * TASK_PACK_ROW_GAP,
    };
  }

  return packed;
}
