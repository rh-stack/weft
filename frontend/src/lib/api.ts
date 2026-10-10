export type Rect = { x: number; y: number; w: number; h: number };
export type Area = { id: string; title: string; rect: Rect };
export type NodeLayout = { id: string; x: number; y: number; areaId?: string | null; parentId?: string | null };
export type Edge = {
  id: string;
  from: string;
  to: string;
  label?: string | null;
  sourceHandle?: string | null;
  targetHandle?: string | null;
};
export const NODE_COLORS = [
  { name: 'Peach', value: '#FED7AA' },
  { name: 'Butter', value: '#FDE68A' },
  { name: 'Mint', value: '#A7F3D0' },
  { name: 'Sky', value: '#BAE6FD' },
  { name: 'Lilac', value: '#DDD6FE' },
];

export type NodeFull = {
  id: string;
  title: string;
  type: string;
  color?: string | null;
  fields: Record<string, unknown>;
  body: string;
  x: number;
  y: number;
  areaId?: string | null;
  parentId?: string | null;
};
export type StateResponse = { areas: Area[]; nodes: NodeFull[]; edges: Edge[]; rev: number };

export async function fetchState(): Promise<StateResponse> {
  const r = await fetch('/api/state');
  if (!r.ok) throw new Error(`load failed: ${r.status}`);
  return r.json();
}

export async function saveState(s: StateResponse): Promise<number> {
  const r = await fetch('/api/state', {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(s),
  });
  if (!r.ok) throw new Error(`save failed: ${r.status}`);
  const j = await r.json();
  return j.rev as number;
}

export async function uploadAsset(name: string, file: File | Blob): Promise<string> {
  const r = await fetch(`/api/assets/${encodeURIComponent(name)}`, { method: 'PUT', body: file });
  if (!r.ok) throw new Error(`upload failed: ${r.status}`);
  const j = await r.json();
  return j.name as string;
}

export function slug(s: string): string {
  return s.toLowerCase().trim().replace(/[^a-z0-9]+/g, '-').replace(/(^-|-$)/g, '') || 'node';
}
