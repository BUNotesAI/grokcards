import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type Dispatch,
  type MouseEvent as ReactMouseEvent,
  type RefObject,
  type SetStateAction,
} from "react";
import type { Position } from "@/bindings";
import type { useViewport } from "@/components/keysight/useViewport";
import type { useLayoutActions } from "@/hooks/useLayoutActions";
import type { WhiteboardData } from "@/components/keysight/hooks/useWhiteboardData";
import type { EntityKind } from "@/components/keysight/types";
import { deriveAllKinds } from "@/components/keysight/lib/deriveAllKinds";

/** 拖拽阈值 — 小于此距离视为 click 而非 drag(屏幕像素) */
const DRAG_THRESHOLD = 4;

// dragInfo 用 discriminated union 表达"拖单 entity 与拖 section 多 entity"两类互斥状态:
// "single" 拿到被拖 DOM element(快路径直写 transform);"section" 拿到一组 id 与各自原位置。
// 让两类共享字段以外的部分各自封闭,避免互相参数泄漏到对方分支。
type DragInfo =
  | {
      kind: "single";
      id: string;
      el: HTMLElement;
      startX: number;
      startY: number;
      origX: number;
      origY: number;
      didDrag: boolean;
    }
  | {
      kind: "section";
      id: string;
      startX: number;
      startY: number;
      origPositions: Record<string, Position>;
      movedIds: string[];
      didDrag: boolean;
    };

export interface UseDragInteractionDeps {
  viewport: ReturnType<typeof useViewport>;
  layouts: ReturnType<typeof useLayoutActions>;
  data: WhiteboardData;
  currentWhiteboardId: string;
}

export interface UseDragInteractionResult {
  handleDragStart: (e: ReactMouseEvent, entityId: string) => void;
  // 暴露给 useEntityEditing / handleToggleExpand / jump 等"刚拖完别 click"判断用。
  lastDidDragRef: RefObject<boolean>;
  // 暴露给 useCreationModals.handlePackTasks 用(pack 把多 task 位置批量塞进 local override)。
  localPositions: Record<string, Position>;
  setLocalPositions: Dispatch<SetStateAction<Record<string, Position>>>;
  // hook 内部 own localPositions,自然成为 effectivePositions(server + local override)的唯一计算源。
  // 暴露给 GraphView 让 allEntities / allDimensions / forceVisibleIds / 各 jump handler 共享同一份。
  effectivePositions: Record<string, Position>;
}

export function useDragInteraction(deps: UseDragInteractionDeps): UseDragInteractionResult {
  const { viewport, layouts, data, currentWhiteboardId } = deps;
  // 自己 derive 一份 allKinds(供 mousedown 时按 kind 分支处理)— 与 useEntityIndexes 是
  // 重复算,但避开"useDragInteraction 必须晚于 useEntityIndexes,而 useEntityIndexes
  // 需要 effectivePositions 又来自 useDragInteraction"的循环依赖。
  const allKinds = useMemo<Record<string, EntityKind>>(
    () => deriveAllKinds(data),
    [data.cards, data.notes, data.tasks, data.questions, data.sections, data.aliases],
  );

  // 拖拽状态:ref 存储启动时的位置/屏幕坐标 + 被拖拽节点的 DOM 引用
  // mousemove 时直接 imperative 更新 el.style.transform,绕过 React 重渲染延迟
  // 同时也调用 setLocalPositions 让 React state 跟上(保持单向数据流一致性)
  const dragInfoRef = useRef<DragInfo | null>(null);
  const lastDidDragRef = useRef(false);
  const [localPositions, setLocalPositions] = useState<Record<string, Position>>({});

  // 有效位置 = 服务器位置 + 本地覆盖(拖拽中的实时位置)
  const effectivePositions = useMemo(
    () => ({ ...data.positions, ...localPositions }),
    [data.positions, localPositions],
  );

  // ref 镜像:closure 内通过 ref 读最新 memo 值,避免 mousemove listener 每次 effective 变更重订
  const effectivePositionsRef = useRef(effectivePositions);
  useEffect(() => {
    effectivePositionsRef.current = effectivePositions;
  }, [effectivePositions]);

  const allKindsRef = useRef(allKinds);
  useEffect(() => {
    allKindsRef.current = allKinds;
  }, [allKinds]);

  const sectionsRef = useRef(data.sections);
  useEffect(() => {
    sectionsRef.current = data.sections;
  }, [data.sections]);

  // 拖拽:节点 mousedown 触发(stable reference)
  // 捕获被拖动节点的 wrapper DOM(e.currentTarget),mousemove 时直接更新它的 transform
  const handleDragStart = useCallback((e: ReactMouseEvent, entityId: string) => {
    if (allKindsRef.current[entityId] === "section") {
      const section = sectionsRef.current.find((item) => item.id === entityId);
      if (!section) return;

      const movedIds = section.cardIds.filter((id) => effectivePositionsRef.current[id] != null);
      const effectiveMovedIds = movedIds.length > 0 ? movedIds : [entityId];
      const origPositions: Record<string, Position> = {};
      for (const id of effectiveMovedIds) {
        const pos = effectivePositionsRef.current[id];
        if (pos) origPositions[id] = pos;
      }
      if (Object.keys(origPositions).length === 0) return;

      dragInfoRef.current = {
        kind: "section",
        id: entityId,
        startX: e.clientX,
        startY: e.clientY,
        origPositions,
        movedIds: Object.keys(origPositions),
        didDrag: false,
      };
      return;
    }

    const pos = effectivePositionsRef.current[entityId];
    if (!pos) return;
    dragInfoRef.current = {
      kind: "single",
      id: entityId,
      el: e.currentTarget as HTMLElement,
      startX: e.clientX,
      startY: e.clientY,
      origX: pos.x,
      origY: pos.y,
      didDrag: false,
    };
  }, []);

  // 全局 mousemove / mouseup — 拖拽中实时更新位置,释放时持久化
  useEffect(() => {
    const onMove = (e: MouseEvent) => {
      const info = dragInfoRef.current;
      if (!info) return;
      const rawDx = e.clientX - info.startX;
      const rawDy = e.clientY - info.startY;
      if (!info.didDrag) {
        if (rawDx * rawDx + rawDy * rawDy < DRAG_THRESHOLD * DRAG_THRESHOLD) return;
        info.didDrag = true;
      }
      // 按当前 zoom 缩放屏幕坐标差 → 世界坐标差
      const zoom = viewport.state.zoom;
      const dx = rawDx / zoom;
      const dy = rawDy / zoom;
      if (info.kind === "single") {
        const newX = info.origX + dx;
        const newY = info.origY + dy;
        // Fast path:直接更新 DOM transform,每帧都立即响应
        // 不依赖 React 的 render 调度,避免任何 batching/scheduling 延迟导致视觉滞后
        info.el.style.transform = `translate3d(${newX}px, ${newY}px, 0)`;
        // Slow path:同步更新 React state 让其他依赖(section bounds 等)跟上
        setLocalPositions((prev) => ({
          ...prev,
          [info.id]: { x: newX, y: newY },
        }));
        return;
      }

      setLocalPositions((prev) => {
        const next = { ...prev };
        for (const id of info.movedIds) {
          const orig = info.origPositions[id];
          next[id] = {
            x: orig.x + dx,
            y: orig.y + dy,
          };
        }
        return next;
      });
    };

    const onUp = () => {
      const info = dragInfoRef.current;
      if (!info) return;
      dragInfoRef.current = null;
      lastDidDragRef.current = info.didDrag; // 给 onClick 用
      if (!info.didDrag) return; // 没移动足够距离 — click 不持久化
      // 持久化到 DB — 使用 setLocalPositions 的回调拿到最新值
      setLocalPositions((prev) => {
        const persistIds = info.kind === "single" ? [info.id] : info.movedIds;
        const writes: Promise<unknown>[] = [];
        for (const id of persistIds) {
          const pos = prev[id];
          if (!pos) continue;
          writes.push(
            layouts.setPositionWithoutInvalidate(currentWhiteboardId, id, pos.x, pos.y),
          );
        }

        if (writes.length > 0) {
          Promise.all(writes)
            .then(() => {
              layouts.invalidatePositions();
            })
            .catch((err) => console.error("拖拽持久化失败:", err));
        }
        return prev;
      });
    };

    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
    return () => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
    };
  }, [viewport.state.zoom, currentWhiteboardId, layouts]);

  // 服务器位置回来后,清理本地覆盖(避免 stale override)
  useEffect(() => {
    const toDelete: string[] = [];
    for (const [id, localPos] of Object.entries(localPositions)) {
      const serverPos = data.positions[id];
      if (
        serverPos &&
        Math.abs(serverPos.x - localPos.x) < 0.5 &&
        Math.abs(serverPos.y - localPos.y) < 0.5
      ) {
        toDelete.push(id);
      }
    }
    if (toDelete.length > 0) {
      setLocalPositions((prev) => {
        const next = { ...prev };
        for (const id of toDelete) delete next[id];
        return next;
      });
    }
  }, [data.positions, localPositions]);

  return {
    handleDragStart,
    lastDidDragRef,
    localPositions,
    setLocalPositions,
    effectivePositions,
  };
}
