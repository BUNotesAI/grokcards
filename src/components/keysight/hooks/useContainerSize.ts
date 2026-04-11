import { useEffect, useState, type RefObject } from "react";

/** 容器尺寸 */
export interface ContainerSize {
  width: number;
  height: number;
}

/**
 * 监听容器尺寸变化的 hook。
 *
 * 基于 ResizeObserver，返回容器当前 width/height。
 * 视口裁剪需要知道容器尺寸才能计算世界坐标范围。
 */
export function useContainerSize(ref: RefObject<HTMLElement | null>): ContainerSize {
  const [size, setSize] = useState<ContainerSize>({ width: 0, height: 0 });

  useEffect(() => {
    const el = ref.current;
    if (!el) return;

    const observer = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const { width, height } = entry.contentRect;
        setSize({ width, height });
      }
    });

    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);

  return size;
}
