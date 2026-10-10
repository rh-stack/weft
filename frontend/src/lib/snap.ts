export type SnapSize = { w: number; h: number };
export type SnapGuide = { orientation: 'v' | 'h'; pos: number; from: number; to: number };
export type SnapResult = { dx: number; dy: number; guides: SnapGuide[] };
export type SnapBox = { x: number; y: number; w: number; h: number };

export const SNAP_SCREEN_PX = 8;
export const SNAP_PAD = 24;
export const FALLBACK_SIZE: SnapSize = { w: 240, h: 100 };

function anchorsX(b: SnapBox): number[] {
  return [b.x, b.x + b.w / 2, b.x + b.w];
}

function anchorsY(b: SnapBox): number[] {
  return [b.y, b.y + b.h / 2, b.y + b.h];
}

function sizeOf(id: string, sizes: Map<string, SnapSize>, node?: any): SnapSize {
  const cached = sizes.get(id);
  if (cached) return cached;
  const mw = Number(node?.measured?.width ?? node?.width);
  const mh = Number(node?.measured?.height ?? node?.height);
  return {
    w: Number.isFinite(mw) && mw > 0 ? mw : FALLBACK_SIZE.w,
    h: Number.isFinite(mh) && mh > 0 ? mh : FALLBACK_SIZE.h,
  };
}

function dragBounds(
  draggedIds: Set<string>,
  nodes: any[],
  sizes: Map<string, SnapSize>,
): SnapBox | null {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let found = false;
  for (const n of nodes) {
    if (!draggedIds.has(n?.id)) continue;
    const s = sizeOf(n.id, sizes, n);
    const x = Number(n?.position?.x);
    const y = Number(n?.position?.y);
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    found = true;
    if (x < minX) minX = x;
    if (y < minY) minY = y;
    if (x + s.w > maxX) maxX = x + s.w;
    if (y + s.h > maxY) maxY = y + s.h;
  }
  if (!found) return null;
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

function otherBoxes(
  draggedIds: Set<string>,
  nodes: any[],
  sizes: Map<string, SnapSize>,
): SnapBox[] {
  const out: SnapBox[] = [];
  for (const n of nodes) {
    if (!n || draggedIds.has(n.id)) continue;
    const s = sizeOf(n.id, sizes, n);
    const x = Number(n?.position?.x);
    const y = Number(n?.position?.y);
    if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
    out.push({ x, y, w: s.w, h: s.h });
  }
  return out;
}

function bestShift(dragAnchors: number[], otherAnchors: number[][], threshold: number): number {
  let best = 0;
  let bestAbs = Infinity;
  for (const d of dragAnchors) {
    for (const list of otherAnchors) {
      for (const o of list) {
        const diff = o - d;
        const abs = Math.abs(diff);
        if (abs <= threshold && abs < bestAbs) {
          bestAbs = abs;
          best = diff;
        }
      }
    }
  }
  return bestAbs === Infinity ? 0 : best;
}

function guideSpanVertical(pos: number, drag: SnapBox, others: SnapBox[]): SnapGuide {
  let from = drag.y;
  let to = drag.y + drag.h;
  for (const o of others) {
    const xs = anchorsX(o);
    if (xs.some((v) => Math.abs(v - pos) < 0.5)) {
      if (o.y < from) from = o.y;
      if (o.y + o.h > to) to = o.y + o.h;
    }
  }
  return { orientation: 'v', pos, from: from - SNAP_PAD, to: to + SNAP_PAD };
}

function guideSpanHorizontal(pos: number, drag: SnapBox, others: SnapBox[]): SnapGuide {
  let from = drag.x;
  let to = drag.x + drag.w;
  for (const o of others) {
    const ys = anchorsY(o);
    if (ys.some((v) => Math.abs(v - pos) < 0.5)) {
      if (o.x < from) from = o.x;
      if (o.x + o.w > to) to = o.x + o.w;
    }
  }
  return { orientation: 'h', pos, from: from - SNAP_PAD, to: to + SNAP_PAD };
}

export function computeSnap(
  draggedIds: Set<string>,
  nodes: any[],
  sizes: Map<string, SnapSize>,
  threshold: number,
): SnapResult {
  const guides: SnapGuide[] = [];
  if (draggedIds.size === 0 || threshold <= 0) return { dx: 0, dy: 0, guides };
  const drag = dragBounds(draggedIds, nodes, sizes);
  if (!drag) return { dx: 0, dy: 0, guides };
  const others = otherBoxes(draggedIds, nodes, sizes);
  if (others.length === 0) return { dx: 0, dy: 0, guides };

  const dragX = anchorsX(drag);
  const dragY = anchorsY(drag);
  const otherX = others.map(anchorsX);
  const otherY = others.map(anchorsY);

  const dx = bestShift(dragX, otherX, threshold);
  const dy = bestShift(dragY, otherY, threshold);

  if (dx !== 0) {
    const moved = { ...drag, x: drag.x + dx };
    const pos = anchorsX(moved).find((v) => otherX.some((list) => list.some((o) => Math.abs(o - v) < 0.5))) ?? moved.x;
    guides.push(guideSpanVertical(pos, { ...drag, x: drag.x + dx }, others));
  }
  if (dy !== 0) {
    const moved = { ...drag, y: drag.y + dy };
    const pos = anchorsY(moved).find((v) => otherY.some((list) => list.some((o) => Math.abs(o - v) < 0.5))) ?? moved.y;
    guides.push(guideSpanHorizontal(pos, { ...drag, y: drag.y + dy }, others));
  }
  return { dx, dy, guides };
}

export function readNodeSizesFromDom(root: ParentNode): Map<string, SnapSize> {
  const out = new Map<string, SnapSize>();
  const els = root.querySelectorAll('.svelte-flow__node[data-id]');
  els.forEach((el) => {
    const id = (el as HTMLElement).dataset?.id;
    if (!id) return;
    const w = (el as HTMLElement).offsetWidth;
    const h = (el as HTMLElement).offsetHeight;
    if (Number.isFinite(w) && Number.isFinite(h) && w > 0 && h > 0) out.set(id, { w, h });
  });
  return out;
}
