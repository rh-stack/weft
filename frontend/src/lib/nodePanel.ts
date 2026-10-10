import { writable, get } from 'svelte/store';

// Attached node action flap. Lives in a shared store so opening or
// closing it never rebuilds the flow arrays (which would disturb the edges).
// Both App.svelte and NodeCard.svelte read it directly.
export type NodePanelSide = 'below' | 'above';
export type NodePanel = { id: string; side: NodePanelSide } | null;

export const nodePanel = writable<NodePanel>(null);

export function setNodePanel(p: NodePanel) {
  const cur = get(nodePanel);
  const a = cur ? cur.id + cur.side : '';
  const b = p ? p.id + p.side : '';
  if (a === b) return;
  nodePanel.set(p);
}
