<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { SvelteFlow, Controls, ControlButton, Background, BackgroundVariant, ConnectionLineType } from '@xyflow/svelte';
  import '@xyflow/svelte/dist/style.css';
  import NodeCard from './lib/NodeCard.svelte';
  import LockSync from './lib/LockSync.svelte';
  import FlowHotkeys from './lib/FlowHotkeys.svelte';
  import { flip } from 'svelte/animate';
  import { fetchState, saveState, slug, NODE_COLORS, type StateResponse, type Area, type Edge, type NodeFull } from './lib/api';
  // Milkdown is heavy, so it loads on first notes open. Canvas stays lean.
  let EditorComp: any = null;
  function ensureEditor() {
    if (!EditorComp) import('./lib/NodeEditor.svelte').then((m) => (EditorComp = m.default));
  }

  const nodeTypes = { card: NodeCard };
  let flowNodes: any[] = [];
  let flowEdges: any[] = [];

  let state: StateResponse | null = null;
  let areas: Area[] = [];
  let viewport = { x: 0, y: 0, zoom: 1 };
  let error = '';
  let savedState: 'saved' | 'editing' | 'saving' | 'error' = 'loading' as any;
  let rev = 0;
  let search = '';
  let searchEl: HTMLInputElement | null = null;
  let matches: { id: string; title: string; type: string }[] = [];
  let selectedId: string | null = null;
  let selectedEdgeId: string | null = null;
  let locked = false;
  // key-value row drag reorder
  let dragField = -1;
  let dropAt = -1;
  function startFieldDrag(e: DragEvent, i: number) {
    dragField = i;
    if (!e.dataTransfer) return;
    e.dataTransfer.effectAllowed = 'move';
    const row = (e.currentTarget as HTMLElement).closest('.kv');
    if (row) e.dataTransfer.setDragImage(row, 12, 12);
  }
  function endFieldDrag() {
    dragField = -1;
    dropAt = -1;
  }
  // The insertion point is the row boundary nearest the cursor.
  function fieldsDragOver(e: DragEvent) {
    e.preventDefault();
    const rows = [...(e.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('.kv')];
    const first = rows.findIndex((el) => e.clientY < el.getBoundingClientRect().top + el.offsetHeight / 2);
    dropAt = first < 0 ? rows.length : first;
  }
  function fieldsDragLeave(e: DragEvent) {
    if (!(e.currentTarget as HTMLElement).contains(e.relatedTarget as Node | null)) dropAt = -1;
  }
  // A line shows at a boundary. No line shows where the row would stay in place.
  function isDropLine(from: number, at: number, boundary: number) {
    return from >= 0 && at === boundary && boundary !== from && boundary !== from + 1;
  }
  function fieldDrop() {
    const from = dragField;
    let to = dropAt;
    endFieldDrag();
    if (from < 0 || to < 0) return;
    if (from < to) to -= 1;
    if (to === from) return;
    const rows = [...fieldRows];
    const [moved] = rows.splice(from, 1);
    rows.splice(to, 0, moved);
    fieldRows = rows;
    queueNotesSave();
  }
  let selectedAreaId: string | null = null;
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let dirty = false;
  let dragging = false;
  let pendingReload = false;
  let lastClick = { id: '', t: 0 };
  let flowWrap: HTMLDivElement;
  let showHelp = false;

  // right-click menu
  let menu: { x: number; y: number; kind: 'pane' | 'node' | 'area'; nodeId?: string; areaId?: string } | null = null;
  // edge label inline edit + last mouse spot on canvas
  let editingEdge: { id: string; x: number; y: number } | null = null;
  let edgeLabelVal = '';
  let mouse = { x: 0, y: 0 };
  function commitEdgeLabel() {
    if (!state || !editingEdge) {
      editingEdge = null;
      return;
    }
    const t = edgeLabelVal.trim();
    state.edges = state.edges.map((e) => (e.id === editingEdge!.id ? { ...e, label: t } : e));
    editingEdge = null;
    rebuildFlow();
    scheduleSave();
  }

  // area inline rename
  let renamingArea: string | null = null;
  let renameVal = '';
  function commitRename() {
    const t = renameVal.trim();
    if (renamingArea && t) {
      areas = areas.map((a) => (a.id === renamingArea ? { ...a, title: t } : a));
      scheduleSave();
    }
    renamingArea = null;
  }

  // notes popover: single Milkdown markdown mode with autosave
  let openId: string | null = null;
  let editTitle = '';
  let editType = '';
  let editParent = '';
  let editColor: string | null = null;
  let fieldRows: { k: string; v: string }[] = [];
  let editBody = '';
  let notesTimer: ReturnType<typeof setTimeout> | null = null;

  // area move / resize via corner grips
  let areaOp: { id: string; mode: 'move' | 'resize'; sx: number; sy: number; orig: { x: number; y: number; w: number; h: number } } | null = null;

  // Nodes that can become the parent of a node. The node and its descendants are skipped, so no cycle forms.
  function parentChoices(s: StateResponse | null, id: string | null): { id: string; title: string }[] {
    if (!s || !id) return [];
    const blocked = new Set<string>([id]);
    let grew = true;
    while (grew) {
      grew = false;
      for (const n of s.nodes) {
        if (n.parentId && blocked.has(n.parentId) && !blocked.has(n.id)) {
          blocked.add(n.id);
          grew = true;
        }
      }
    }
    return s.nodes.filter((n) => !blocked.has(n.id)).map((n) => ({ id: n.id, title: n.title }));
  }

  function childrenOf(id: string) {
    if (!state) return [];
    return state.nodes.filter((n) => n.parentId === id).map((n) => ({ id: n.id, title: n.title }));
  }

  function snapshot(): string {
    if (!state) return '';
    return JSON.stringify([state.nodes.map((n) => [n.id, n.x, n.y]), areas.map((a) => [a.id, a.rect.x, a.rect.y, a.rect.w, a.rect.h])]);
  }
  let lastSynced = '';

  function uniqueId(base: string, taken: (id: string) => boolean): string {
    let id = base || 'node';
    let i = 2;
    while (taken(id)) id = `${base}-${i++}`;
    return id;
  }

  function rebuildFlow() {
    if (!state) return;
    const q = search.trim().toLowerCase();
    flowNodes = state.nodes.map((n) => ({
      id: n.id,
      type: 'card',
      position: { x: n.x, y: n.y },
      data: {
        id: n.id,
        title: n.title,
        ntype: n.type,
        color: n.color ?? null,
        fields: n.fields ?? {},
        children: childrenOf(n.id),
        dimmed: q ? !(n.title.toLowerCase().includes(q) || n.id.includes(q)) : false,
      },
    }));
    flowEdges = state.edges.map((e) => ({
      id: e.id,
      type: 'smoothstep',
      source: e.from,
      target: e.to,
      label: e.label ?? '',
      sourceHandle: e.sourceHandle ?? null,
      targetHandle: e.targetHandle ?? null,
      pathOptions: { borderRadius: 8 },
    }));
    lastSynced = snapshot();
  }

  async function load() {
    try {
      state = await fetchState();
      areas = state.areas;
      rev = state.rev;
      dirty = false;
      pendingReload = false;
      refreshMatches();
      rebuildFlow();
      savedState = 'saved';
      if (openId) openNotes(openId, true);
    } catch (e: any) {
      error = String(e?.message ?? e);
      savedState = 'error';
    }
  }

  function scheduleSave() {
    if (!state) return;
    state.areas = areas;
    dirty = true;
    savedState = 'editing';
    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(doSave, 800);
  }

  async function doSave() {
    if (!state || !dirty) return;
    try {
      savedState = 'saving';
      state.areas = areas;
      rev = await saveState({ ...state, rev });
      dirty = false;
      savedState = 'saved';
    } catch (e: any) {
      error = String(e?.message ?? e);
      savedState = 'error';
    }
  }

  function syncNodesFromFlow(): boolean {
    if (!state) return false;
    const byId = new Map(flowNodes.map((n: any) => [n.id, n]));
    for (const n of state.nodes) {
      const f = byId.get(n.id) as any;
      if (f?.position) {
        n.x = Math.round(f.position.x);
        n.y = Math.round(f.position.y);
      }
    }
    const now = snapshot();
    if (now === lastSynced) return false;
    lastSynced = now;
    return true;
  }

  let syncTimer: ReturnType<typeof setTimeout> | null = null;

  function onDragStart() {
    dragging = true;
    menu = null;
    if (syncTimer) clearTimeout(syncTimer);
  }

  function onDragStop() {
    dragging = false;
    // The flow commits final positions to the bound array just after
    // dragstop fires, so read them on the next tick.
    deferredSync();
    if (pendingReload) load();
  }

  function deferredSync() {
    if (syncTimer) clearTimeout(syncTimer);
    syncTimer = setTimeout(() => {
      if (dragging) return;
      if (syncNodesFromFlow()) scheduleSave();
    }, 120);
  }

  function onConnect(c: any) {
    if (!state || !c?.source || !c?.target || c.source === c.target) return;
    state.edges = [
      ...state.edges,
      {
        id: `e-${Date.now().toString(36)}`,
        from: c.source,
        to: c.target,
        label: '',
        sourceHandle: c.sourceHandle ?? null,
        targetHandle: c.targetHandle ?? null,
      },
    ];
    rebuildFlow();
    scheduleSave();
  }

  function onNodeClick(e: any) {
    const n = e?.node;
    if (!n) return;
    selectedId = n.id;
    selectedEdgeId = null;
    selectedAreaId = null;
    // double click opens notes
    const now = Date.now();
    if (lastClick.id === n.id && now - lastClick.t < 350) openNotes(n.id);
    lastClick = { id: n.id, t: now };
  }

  function onEdgeClick(e: any) {
    if (e?.edge) {
      selectedEdgeId = e.edge.id;
      selectedId = null;
      selectedAreaId = null;
    }
  }

  function onPaneClick() {
    selectedId = null;
    selectedEdgeId = null;
    selectedAreaId = null;
    menu = null;
  }

  function flowPos(clientX: number, clientY: number) {
    const r = flowWrap.getBoundingClientRect();
    return {
      x: Math.round((clientX - r.left - viewport.x) / viewport.zoom),
      y: Math.round((clientY - r.top - viewport.y) / viewport.zoom),
    };
  }

  function onStageMenu(e: MouseEvent) {
    e.preventDefault();
    const t = e.target as HTMLElement;
    const nodeEl = t.closest ? (t.closest('.svelte-flow__node') as HTMLElement | null) : null;
    const id = nodeEl?.dataset?.id;
    if (id && !id.startsWith('area-')) {
      selectedId = id;
      selectedEdgeId = null;
      menu = { x: e.clientX, y: e.clientY, kind: 'node', nodeId: id };
    } else {
      menu = { x: e.clientX, y: e.clientY, kind: 'pane' };
    }
  }

  function addNodeAt(clientX: number, clientY: number, title: string, type: string) {
    if (!state) return;
    const id = uniqueId(slug(title || 'new-node'), (x) => state!.nodes.some((n) => n.id === x));
    const p = flowPos(clientX, clientY);
    state.nodes = [
      ...state.nodes,
      { id, title: (title || 'New node').trim(), type: type || 'note', color: null, fields: {}, body: `# ${(title || 'New node').trim()}\n`, x: p.x - 120, y: p.y - 40, areaId: null, parentId: null },
    ];
    selectedId = id;
    menu = null;
    refreshMatches();
    rebuildFlow();
    scheduleSave();
    openNotes(id);
  }

  function addAreaAt(clientX: number, clientY: number, title: string) {
    if (!state) return;
    const id = uniqueId(slug(title || 'new-area'), (x) => areas.some((a) => a.id === x));
    const p = flowPos(clientX, clientY);
    areas = [...areas, { id, title: (title || 'New area').trim(), rect: { x: p.x - 200, y: p.y - 100, w: 480, h: 360 } }];
    menu = null;
    scheduleSave();
  }

  // Undo for canvas deletes. Keeps the last 3 deleted nodes, areas, or edges.
  type Deleted =
    | { kind: 'node'; node: NodeFull; edges: Edge[]; children: string[] }
    | { kind: 'area'; area: Area }
    | { kind: 'edge'; edge: Edge };
  const UNDO_LIMIT = 3;
  let undoStack: Deleted[] = [];

  function remember(d: Deleted) {
    undoStack = [...undoStack, d].slice(-UNDO_LIMIT);
  }

  function deleteSelection() {
    if (!state) return;
    if (selectedEdgeId) {
      const edge = state.edges.find((e) => e.id === selectedEdgeId);
      if (edge) remember({ kind: 'edge', edge });
      state.edges = state.edges.filter((e) => e.id !== selectedEdgeId);
      selectedEdgeId = null;
    } else if (selectedId) {
      const id = selectedId;
      const node = state.nodes.find((n) => n.id === id);
      if (node) {
        remember({
          kind: 'node',
          node,
          edges: state.edges.filter((e) => e.from === id || e.to === id),
          children: state.nodes.filter((n) => n.parentId === id).map((n) => n.id),
        });
      }
      state.nodes = state.nodes.filter((n) => n.id !== id);
      state.nodes = state.nodes.map((n) => (n.parentId === id ? { ...n, parentId: null } : n));
      state.edges = state.edges.filter((e) => e.from !== id && e.to !== id);
      if (openId === id) openId = null;
      selectedId = null;
    } else if (selectedAreaId) {
      const area = areas.find((a) => a.id === selectedAreaId);
      if (area) remember({ kind: 'area', area });
      areas = areas.filter((a) => a.id !== selectedAreaId);
      selectedAreaId = null;
    } else return;
    refreshMatches();
    rebuildFlow();
    scheduleSave();
  }

  // Restores the newest deleted item. Skips the restore when its id is taken again.
  function undoDelete() {
    const d = undoStack[undoStack.length - 1];
    if (!state || !d || openId || editingEdge) return;
    undoStack = undoStack.slice(0, -1);
    selectedId = null;
    selectedEdgeId = null;
    selectedAreaId = null;
    if (d.kind === 'area') {
      if (areas.some((a) => a.id === d.area.id)) return;
      areas = [...areas, d.area];
      selectedAreaId = d.area.id;
    } else if (d.kind === 'edge') {
      if (state.edges.some((e) => e.id === d.edge.id)) return;
      state.edges = [...state.edges, d.edge];
      selectedEdgeId = d.edge.id;
    } else {
      if (state.nodes.some((n) => n.id === d.node.id)) return;
      const children = new Set(d.children);
      state.nodes = [...state.nodes.map((n) => (children.has(n.id) ? { ...n, parentId: d.node.id } : n)), d.node];
      state.edges = [...state.edges, ...d.edges];
      selectedId = d.node.id;
    }
    refreshMatches();
    rebuildFlow();
    scheduleSave();
  }

  function isUndoKey(e: KeyboardEvent) {
    return (e.ctrlKey || e.metaKey) && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'z';
  }

  // With a note open, Tab moves focus into the editor first.
  function tabIntoEditor(e: KeyboardEvent) {
    if (e.key !== 'Tab' || e.shiftKey || !openId) return;
    const editor = document.querySelector<HTMLElement>('.milk .ProseMirror');
    if (!editor || document.activeElement === editor) return;
    e.preventDefault();
    editor.focus();
  }

  function onKey(e: KeyboardEvent) {
    tabIntoEditor(e);
    if (e.key === 'Escape' && showHelp) {
      showHelp = false;
      return;
    }
    const t = document.activeElement as HTMLElement | null;
    const typing = t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
    if (typing) {
      if (e.key === 'Escape' && t === searchEl && search) {
        search = '';
        matches = [];
        onSearchInput();
        searchEl?.blur();
      } else if (e.key === 'Escape') (t as HTMLElement).blur();
      return;
    }
    if (e.key === '?' && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      showHelp = !showHelp;
      return;
    }
    if (showHelp) return;
    if (isUndoKey(e)) {
      e.preventDefault();
      undoDelete();
      return;
    }
    if (e.key === '/' && !openId) {
      e.preventDefault();
      searchEl?.focus();
      return;
    }
    if ((e.key === 'Delete' || e.key === 'Backspace') && !openId && !editingEdge) {
      e.preventDefault();
      deleteSelection();
    }
    if ((e.key === 'r' || e.key === 'R') && !openId && !editingEdge && selectedEdgeId) {
      e.preventDefault();
      const edge = state?.edges.find((x) => x.id === selectedEdgeId);
      editingEdge = { id: selectedEdgeId, x: mouse.x, y: mouse.y };
      edgeLabelVal = edge?.label ?? '';
      requestAnimationFrame(() => document.querySelector<HTMLElement>('.edgelabel-edit')?.focus());
    }
    if ((e.key === 'd' || e.key === 'D') && !openId && !editingEdge && selectedEdgeId && !e.ctrlKey && !e.metaKey && !e.altKey) {
      e.preventDefault();
      deleteSelection();
    }
    if (e.key === 'Escape') {
      menu = null;
      if (openId) closeNotes();
    }
  }

  function refreshMatches() {
    const q = search.trim().toLowerCase();
    matches = !state || !q ? [] : state.nodes.filter((n) => n.title.toLowerCase().includes(q) || n.id.includes(q)).slice(0, 8).map((n) => ({ id: n.id, title: n.title, type: n.type }));
  }

  function onSearchInput() {
    refreshMatches();
    const q = search.trim().toLowerCase();
    flowNodes = flowNodes.map((n: any) => ({
      ...n,
      data: { ...n.data, dimmed: q ? !(String(n.data.title).toLowerCase().includes(q) || String(n.id).includes(q)) : false },
    }));
  }

  function focusNode(id: string) {
    const n = flowNodes.find((x: any) => x.id === id);
    if (!n || !flowWrap) return;
    const z = viewport.zoom || 1;
    viewport = { ...viewport, x: flowWrap.clientWidth / 2 - n.position.x * z - 130 * z, y: flowWrap.clientHeight / 2 - n.position.y * z - 60 * z };
    selectedId = id;
    search = '';
    matches = [];
    onSearchInput();
  }

  // notes popover, single Milkdown markdown mode with autosave
  function openNotes(id: string, keep = false) {
    if (!state) return;
    const n = state.nodes.find((x) => x.id === id);
    if (!n) return;
    if (!keep || openId !== id) {
      if (notesTimer) clearTimeout(notesTimer);
      editTitle = n.title;
      editType = n.type;
      editParent = n.parentId ?? '';
      editColor = n.color ?? null;
      fieldRows = Object.entries(n.fields ?? {}).map(([k, v]) => ({ k, v: String(v ?? '') }));
      editBody = n.body ?? '';
    }
    openId = id;
    menu = null;
    ensureEditor();
  }

  function queueNotesSave() {
    if (notesTimer) clearTimeout(notesTimer);
    notesTimer = setTimeout(() => saveNotes(), 800);
  }

  function onEditorChange(md: string) {
    editBody = md;
    queueNotesSave();
  }

  function saveNotes() {
    if (!state || !openId) return;
    const n = state.nodes.find((x) => x.id === openId);
    if (!n) return;
    const fields: Record<string, unknown> = {};
    for (const r of fieldRows) if (r.k.trim()) fields[r.k.trim()] = r.v;
    n.title = editTitle.trim() || n.title;
    n.type = editType.trim() || n.type;
    n.parentId = editParent || null;
    n.color = editColor;
    n.fields = fields;
    n.body = editBody;
    refreshMatches();
    rebuildFlow();
    scheduleSave();
  }

  function closeNotes() {
    if (notesTimer) clearTimeout(notesTimer);
    saveNotes();
    openId = null;
    if (pendingReload) load();
  }

  // area background layer: move and resize grips
  function gripDown(e: PointerEvent, id: string, mode: 'move' | 'resize') {
    if (locked) return;
    e.stopPropagation();
    e.preventDefault();
    const a = areas.find((x) => x.id === id);
    if (!a) return;
    selectedAreaId = id;
    selectedId = null;
    selectedEdgeId = null;
    areaOp = { id, mode, sx: e.clientX, sy: e.clientY, orig: { ...a.rect } };

    window.addEventListener('pointermove', gripMove);
    window.addEventListener('pointerup', gripUp, { once: true });
  }

  function gripMove(e: PointerEvent) {
    if (!areaOp) return;
    const z = viewport.zoom || 1;
    const dx = (e.clientX - areaOp.sx) / z;
    const dy = (e.clientY - areaOp.sy) / z;
    areas = areas.map((a) => {
      if (a.id !== areaOp!.id) return a;
      if (areaOp!.mode === 'move') return { ...a, rect: { ...a.rect, x: Math.round(areaOp!.orig.x + dx), y: Math.round(areaOp!.orig.y + dy) } };
      return { ...a, rect: { ...a.rect, w: Math.max(240, Math.round(areaOp!.orig.w + dx)), h: Math.max(160, Math.round(areaOp!.orig.h + dy)) } };
    });
  }

  function gripUp() {
    window.removeEventListener('pointermove', gripMove);
    areaOp = null;
    scheduleSave();
    if (pendingReload) load();
  }

  function connectWs() {
    try {
      const proto = location.protocol === 'https:' ? 'wss' : 'ws';
      const s = new WebSocket(`${proto}://${location.host}/ws`);
      s.onmessage = async (msg) => {
        try {
          const j = JSON.parse(msg.data);
          if (typeof j.rev === 'number' && j.rev !== rev && !dirty) {
            if (dragging || openId || areaOp) pendingReload = true;
            else await load();
          }
        } catch { /* keep old state */ }
      };
    } catch { /* offline ok */ }
  }

  function onWinDown(e: PointerEvent) {
    if (!menu) return;
    if ((e.button === 2)) return;
    const t = e.target as HTMLElement;
    if (t.closest && t.closest('.ctxmenu')) return;
    menu = null;
  }

  onMount(() => {
    window.addEventListener('open-node', ((e: CustomEvent<string>) => openNotes(e.detail)) as EventListener);
    window.addEventListener('pointerdown', onWinDown, true);
    load();
    connectWs();
  });

  onDestroy(() => {
    window.removeEventListener('pointerdown', onWinDown, true);
  });
</script>

<svelte:window on:keydown={onKey} />

{#if error}<div style="padding:6px 14px;background:#fef3f2;color:#b42318;font-size:12px">{error} <button class="btn" on:click={() => (error = '')}>x</button></div>{/if}

{#if !state}
  <div style="padding:20px">Loading…</div>
{:else}
  <div class="stage" class:locked bind:this={flowWrap} on:contextmenu={onStageMenu} on:mousemove={(e) => (mouse = { x: e.clientX, y: e.clientY })}>
    <SvelteFlow
      bind:nodes={flowNodes}
      bind:edges={flowEdges}
      bind:viewport
      {nodeTypes}
      fitView
      fitViewOptions={{ padding: 0.3 }}
      nodesDraggable={!locked}
      nodesConnectable={!locked}
      elementsSelectable={!locked}
      deleteKey={null}
      defaultEdgeOptions={{ type: 'smoothstep' }}
      connectionLineType={ConnectionLineType.SmoothStep}
      onnodedragstart={onDragStart}
      onnodedragstop={onDragStop}
      onnodeclick={onNodeClick}
      onedgeclick={onEdgeClick}
      onpaneclick={onPaneClick}
      onmovestart={() => (menu = null)}
      onconnect={onConnect}
    >
      <Controls showZoom={false} fitViewOptions={{ padding: 0.3 }}>
        <ControlButton onclick={() => (showHelp = true)} title="Keyboard shortcuts (?)" aria-label="Keyboard shortcuts">?</ControlButton>
      </Controls>
      <Background variant={BackgroundVariant.Dots} />
      <LockSync onlock={(v) => (locked = v)} />
      <FlowHotkeys disabled={openId !== null || showHelp} />
    </SvelteFlow>
    <div class="arealayer">
      {#each areas as a}
        <div
          class="areabox {selectedAreaId === a.id ? 'selected' : ''}"
          style="left:{viewport.x + a.rect.x * viewport.zoom}px;top:{viewport.y + a.rect.y * viewport.zoom}px;width:{a.rect.w * viewport.zoom}px;height:{a.rect.h * viewport.zoom}px;"
          on:contextmenu|preventDefault={(e) => { selectedAreaId = a.id; menu = { x: e.clientX, y: e.clientY, kind: 'area', areaId: a.id }; }}
        >
          {#if !locked}
            <button class="grip" title="Drag to move area" on:pointerdown={(e) => gripDown(e, a.id, 'move')}></button>
          {/if}
          {#if renamingArea === a.id}
            <input
              class="alabel-edit"
              bind:value={renameVal}
              on:blur={commitRename}
              on:keydown={(e) => {
                if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
                if (e.key === 'Escape') renamingArea = null;
              }}
            />
          {:else}
            <button
              class="alabel"
              title="Click to rename"
              on:click={(e) => {
                if (locked) return;
                e.stopPropagation();
                selectedAreaId = a.id;
                renamingArea = a.id;
                renameVal = a.title;
                requestAnimationFrame(() => document.querySelector<HTMLElement>('.alabel-edit')?.focus());
              }}>{a.title}</button
            >
          {/if}
          {#if !locked}
            <button class="rsz" title="Drag to resize" on:pointerdown={(e) => gripDown(e, a.id, 'resize')}></button>
          {/if}
        </div>
      {/each}
    </div>
    <div class="island">
      <div class="island-head">
        <span class="brand"><img src="/logo.svg" alt="Weft logo" width="18" height="18" />Weft</span>
        <span class="status"><span class="dot {savedState}"></span>{savedState === 'saved' ? 'Saved' : savedState === 'editing' ? 'Editing' : savedState === 'saving' ? 'Saving' : savedState}</span>
      </div>
      <input
        placeholder="Search nodes  ( / )"
        bind:this={searchEl}
        bind:value={search}
        on:input={onSearchInput}
        on:keydown={(e) => {
          if (e.key === 'Enter' && matches.length === 1) {
            const id = matches[0].id;
            search = '';
            matches = [];
            onSearchInput();
            (e.target as HTMLInputElement).blur();
            openNotes(id);
          }
        }}
      />
      {#if matches.length}
        <div class="results">
          {#each matches as m}
            <button class="result" on:click={() => focusNode(m.id)}>{m.title} · {m.type}</button>
          {/each}
        </div>
      {/if}
    </div>
  </div>
{/if}

{#if editingEdge}
  <input
    class="edgelabel-edit"
    style="left:{editingEdge.x}px;top:{editingEdge.y}px;"
    bind:value={edgeLabelVal}
    on:blur={commitEdgeLabel}
    on:keydown={(e) => {
      if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
      if (e.key === 'Escape') editingEdge = null;
    }}
  />
{/if}

{#if menu && state}
  {@const m = menu}
  <div class="ctxmenu" style="left:{Math.min(m.x, innerWidth - 240)}px;top:{Math.min(m.y, innerHeight - 260)}px;">
    {#if m.kind === 'pane'}
      <button on:click={() => addNodeAt(m.x, m.y, 'New node', 'note')}>Add node</button>
      <button on:click={() => addAreaAt(m.x, m.y, 'New area')}>Add area</button>
    {:else if m.kind === 'node'}
      <button on:click={() => { if (m.nodeId) openNotes(m.nodeId); }}>Open notes</button>
      <button on:click={() => { const id = m.nodeId; menu = null; if (id) { selectedId = id; deleteSelection(); } }}>Delete</button>
    {:else}
      {#if !locked}
      <button
        on:click={() => {
          const a = areas.find((x) => x.id === m.areaId);
          menu = null;
          if (a) {
            selectedAreaId = a.id;
            renamingArea = a.id;
            renameVal = a.title;
            requestAnimationFrame(() => document.querySelector<HTMLElement>('.alabel-edit')?.focus());
          }
        }}>Rename area</button
      >
      {/if}
      <button
        on:click={() => {
          const id = m.areaId;
          menu = null;
          if (id) {
            selectedAreaId = id;
            deleteSelection();
          }
        }}>Delete area</button
      >
    {/if}
  </div>
{/if}

{#if openId && state}
  {@const n = state.nodes.find((x) => x.id === openId)}
  {#if n}
    <div class="overlay" on:click|self={closeNotes}>
      <div class="sheet">
        <div class="sheet-stripe" style="background: {editColor ?? '#e4e7ec'}"></div>
        <div class="sheet-top">
          <input class="title" bind:value={editTitle} on:input={queueNotesSave} placeholder="Title" title="Click to rename" />
          <button class="btn" on:click={() => (showHelp = true)} title="Keyboard shortcuts (?)">?</button>
        </div>
        <div class="doc">
          <div class="doc-inner">
          <div class="meta-card">
            <label class="meta-item">
              <span class="meta-label">Type</span>
              <input class="type-input" bind:value={editType} on:input={queueNotesSave} placeholder="Type" />
            </label>
            <label class="meta-item">
              <span class="meta-label">Parent</span>
              <select class="parent-select" bind:value={editParent} on:change={queueNotesSave}>
                <option value="">No parent</option>
                {#each parentChoices(state, openId) as p (p.id)}
                  <option value={p.id}>{p.title}</option>
                {/each}
              </select>
            </label>
            <div class="meta-item">
              <span class="meta-label">Color</span>
              <div class="colorrow">
                <button
                  class="sw none {editColor === null ? 'on' : ''}"
                  style="background: #ffffff"
                  title="No color"
                  on:click={() => {
                    editColor = null;
                    queueNotesSave();
                  }}></button
                >
                {#each NODE_COLORS as c}
                  <button
                    class="sw {editColor === c.value ? 'on' : ''}"
                    style="background:{c.value}"
                    title={c.name}
                    on:click={() => {
                      editColor = c.value;
                      queueNotesSave();
                    }}></button>
                {/each}
              </div>
            </div>
          </div>
            <div class="fields" on:dragover={fieldsDragOver} on:dragleave={fieldsDragLeave} on:drop|preventDefault={fieldDrop}>
              {#each fieldRows as r, i (r)}
                <span
                  class="kv"
                  class:dragging={dragField === i}
                  class:drop-above={isDropLine(dragField, dropAt, i)}
                  class:drop-below={isDropLine(dragField, dropAt, i + 1)}
                  animate:flip={{ duration: 180 }}><span
                    class="kvgrip"
                    title="Drag to reorder"
                    draggable="true"
                    on:dragstart={(e) => startFieldDrag(e, i)}
                    on:dragend={endFieldDrag}></span><input
                    placeholder="key"
                    bind:value={r.k}
                    on:input={queueNotesSave}
                  /><input
                    placeholder="value"
                    bind:value={r.v}
                    on:input={queueNotesSave}
                  />
                  <button
                    class="remove"
                    title="Remove field"
                    aria-label="Remove field"
                    on:click={() => {
                      fieldRows = fieldRows.filter((x) => x !== r);
                      queueNotesSave();
                    }}><svg viewBox="0 0 12 12" width="10" height="10" aria-hidden="true"><path d="M2 2l8 8M10 2l-8 8" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" /></svg></button>
                </span>
              {/each}
              <button class="btn" on:click={() => (fieldRows = [...fieldRows, { k: '', v: '' }])}>+ Field</button>
            </div>
            {#key openId}
              {#if EditorComp}
                <svelte:component this={EditorComp} value={editBody} onChange={onEditorChange} />
              {:else}
                <p class="hint">Loading editor…</p>
              {/if}
            {/key}
          </div>
        </div>
      </div>
    </div>
  {/if}
{/if}

{#if showHelp}
  <div class="overlay" on:click|self={() => (showHelp = false)}>
    <div class="help-sheet" role="dialog" aria-modal="true" aria-labelledby="help-title">
      <div class="help-head">
        <h2 id="help-title">Keyboard shortcuts</h2>
        <p class="hint">Press <kbd>?</kbd> or <kbd>Esc</kbd> to close.</p>
      </div>
      <dl class="keys">
        <div class="help-row"><dt><kbd>?</kbd></dt><dd>Show or hide this help</dd></div>
        <div class="help-row"><dt><kbd>Esc</kbd></dt><dd>Close help, menu, or notes</dd></div>
        <div class="help-row"><dt><kbd>/</kbd></dt><dd>Focus search</dd></div>
        <div class="help-row"><dt><kbd>F</kbd></dt><dd>Fit view to all nodes</dd></div>
        <div class="help-row"><dt><kbd>L</kbd></dt><dd>Lock or unlock canvas</dd></div>
        <div class="help-row"><dt><kbd>Delete</kbd> <kbd>⌫</kbd></dt><dd>Delete selected node, area, or edge</dd></div>
        <div class="help-row"><dt><kbd>D</kbd></dt><dd>Delete selected edge</dd></div>
        <div class="help-row"><dt><kbd>R</kbd></dt><dd>Rename selected edge</dd></div>
        <div class="help-row"><dt><kbd>Ctrl</kbd> <kbd>Z</kbd></dt><dd>Undo last delete, keeps last 3</dd></div>
        <div class="help-row"><dt><kbd>Tab</kbd></dt><dd>Jump into notes editor when notes are open</dd></div>
      </dl>
      <p class="hint help-note">Canvas keys pause while you type or while notes are open. Undo works on the canvas only.</p>
    </div>
  </div>
{/if}

<style>
  .stage :global(.svelte-flow) { background: transparent; }
  .stage :global(.svelte-flow__pane) { cursor: grab; }
  .stage :global(.svelte-flow__pane:active) { cursor: grabbing; }
  .stage :global(.svelte-flow__node) { cursor: default; }
  .stage :global(.svelte-flow__node button) { cursor: default; }
  .stage :global(.svelte-flow__handle) { cursor: crosshair; }
  .help-sheet { width: min(460px, 94vw); max-height: 84vh; overflow: auto; background: var(--panel); border-radius: 14px; padding: 18px 22px 14px; box-shadow: 0 24px 64px rgba(0, 0, 0, 0.3); }
  .help-head { margin-bottom: 8px; }
  .help-head h2 { margin: 0 0 2px; font-size: 16px; font-weight: 600; }
  .keys { margin: 0; }
  .help-row { display: grid; grid-template-columns: 150px 1fr; gap: 12px; align-items: center; padding: 7px 0; border-top: 1px solid var(--border); font-size: 13px; }
  .help-row:first-child { border-top: none; }
  .help-row dt { display: flex; align-items: center; gap: 4px; white-space: nowrap; }
  .help-row dd { margin: 0; color: var(--text); }
  .help-note { margin: 12px 0 0; }
  kbd { display: inline-block; min-width: 22px; text-align: center; padding: 1px 6px; border: 1px solid var(--border); border-bottom-width: 2px; border-radius: 6px; background: #f6f7f9; font-family: inherit; font-size: 12px; line-height: 1.5; }
</style>
