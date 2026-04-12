interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface QuadtreeEntry extends Rect {
  id: string;
}

const MAX_ITEMS = 8;
const MAX_DEPTH = 10;

function intersects(a: Rect, b: Rect): boolean {
  return a.x < b.x + b.w && a.x + a.w > b.x && a.y < b.y + b.h && a.y + a.h > b.y;
}

export class Quadtree {
  private readonly bounds: Rect;
  private readonly depth: number;
  private items: QuadtreeEntry[] = [];
  private children: Quadtree[] | null = null;

  constructor(bounds: Rect, depth = 0) {
    this.bounds = bounds;
    this.depth = depth;
  }

  static fromEntries(entries: Iterable<QuadtreeEntry>, worldBounds?: Rect): Quadtree {
    let minX = Infinity;
    let minY = Infinity;
    let maxX = -Infinity;
    let maxY = -Infinity;
    const list: QuadtreeEntry[] = [];

    for (const entry of entries) {
      list.push(entry);
      if (entry.x < minX) minX = entry.x;
      if (entry.y < minY) minY = entry.y;
      if (entry.x + entry.w > maxX) maxX = entry.x + entry.w;
      if (entry.y + entry.h > maxY) maxY = entry.y + entry.h;
    }

    if (list.length === 0) {
      return new Quadtree(worldBounds ?? { x: 0, y: 0, w: 1, h: 1 });
    }

    const pad = 100;
    const bounds = worldBounds ?? {
      x: minX - pad,
      y: minY - pad,
      w: maxX - minX + pad * 2,
      h: maxY - minY + pad * 2,
    };

    const tree = new Quadtree(bounds);
    for (const entry of list) {
      tree.insert(entry);
    }
    return tree;
  }

  insert(entry: QuadtreeEntry): void {
    if (this.children) {
      for (const child of this.children) {
        if (intersects(entry, child.bounds)) {
          child.insert(entry);
        }
      }
      return;
    }

    this.items.push(entry);

    if (this.items.length > MAX_ITEMS && this.depth < MAX_DEPTH) {
      this.subdivide();
    }
  }

  queryRect(left: number, top: number, right: number, bottom: number): string[] {
    const query = { x: left, y: top, w: right - left, h: bottom - top };
    const result = new Set<string>();
    this.collectRect(query, result);
    return Array.from(result);
  }

  private collectRect(query: Rect, result: Set<string>): void {
    if (!intersects(query, this.bounds)) return;

    for (const item of this.items) {
      if (intersects(item, query)) {
        result.add(item.id);
      }
    }

    if (!this.children) return;
    for (const child of this.children) {
      child.collectRect(query, result);
    }
  }

  private subdivide(): void {
    const { x, y, w, h } = this.bounds;
    const halfW = w / 2;
    const halfH = h / 2;

    this.children = [
      new Quadtree({ x, y, w: halfW, h: halfH }, this.depth + 1),
      new Quadtree({ x: x + halfW, y, w: halfW, h: halfH }, this.depth + 1),
      new Quadtree({ x, y: y + halfH, w: halfW, h: halfH }, this.depth + 1),
      new Quadtree({ x: x + halfW, y: y + halfH, w: halfW, h: halfH }, this.depth + 1),
    ];

    const items = this.items;
    this.items = [];
    for (const item of items) {
      this.insert(item);
    }
  }
}
