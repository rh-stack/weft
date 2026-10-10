<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
  import { nodePanel } from './nodePanel';
  export let data: {
    id: string;
    title: string;
    ntype: string;
    color?: string | null;
    fields: Record<string, unknown>;
    children: { id: string; title: string }[];
    dimmed?: boolean;
  };
  export let selected: boolean = false;

  function open(id: string) {
    window.dispatchEvent(new CustomEvent('open-node', { detail: id }));
  }

  $: entries = Object.entries(data.fields ?? {}).slice(0, 6);
  $: flap = $nodePanel && $nodePanel.id === data.id ? $nodePanel.side : null;

  function isLink(v: unknown): boolean {
    return /^https?:\/\//i.test(String(v ?? ''));
  }
</script>

{#snippet actions()}
  <div class="nodepanel">
    <button class="nodrag" on:click|stopPropagation={() => open(data.id)}>Open</button>
    <button class="nodrag danger" on:click|stopPropagation={() => window.dispatchEvent(new CustomEvent('delete-node', { detail: data.id }))}>Delete</button>
  </div>
{/snippet}

<div
  class="card {data.color ? 'tinted' : ''} {selected ? 'selected' : ''} {data.dimmed ? 'dimmed' : ''} {flap ? 'flap-' + flap : ''}"
  style="--tint: {data.color ?? 'var(--panel)'}"
>
  {#if flap === 'above'}{@render actions()}{/if}
  <Handle type="target" position={Position.Top} class="conn" />
  <Handle type="target" position={Position.Left} id="l" class="conn side" />
  <div class="head">
    <span class="titletext">{data.title}</span>
    <span class="type">{data.ntype}</span>
  </div>
  {#if entries.length}
    <dl>
      {#each entries as [k, v]}
        <div class="row">
          <dt>{k}</dt>
          <dd>
            {#if isLink(v)}
              <a class="nodrag" href={String(v)} target="_blank" rel="noopener noreferrer" on:click|stopPropagation>{String(v)}</a>
            {:else}
              {String(v)}
            {/if}
          </dd>
        </div>
      {/each}
    </dl>
  {/if}
  {#if data.children.length}
    <div class="kids">
      {#each data.children as c}
        <button class="kid" on:click|stopPropagation={() => open(c.id)}>{c.title}</button>
      {/each}
    </div>
  {/if}
  <Handle type="source" position={Position.Bottom} class="conn" />
  <Handle type="source" position={Position.Right} id="r" class="conn side" />
  {#if flap === 'below'}{@render actions()}{/if}
</div>

<style>
  .card {
    min-width: 220px;
    max-width: 300px;
    background: var(--tint, var(--panel));
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text);
    box-shadow: var(--shadow-sm);
    cursor: default;
  }
  .card :global(button) { cursor: default; }
  /* Dark mode: a chosen color becomes a dark tint, so the light text stays readable. */
  :global(:root[data-theme='dark']) .card.tinted {
    background: color-mix(in srgb, var(--tint) 30%, var(--panel));
  }
  .card.selected { border-color: #4f7cff; }
  .card.dimmed { opacity: 0.25; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 8px; margin: 4px 0 6px; }
  .type {
    flex: none;
    font-size: 10px; line-height: 18px; white-space: nowrap; text-transform: uppercase; letter-spacing: 0.06em;
    color: var(--muted); background: var(--chip); border-radius: 4px; padding: 1px 6px;
  }
  .titletext {
    flex: 1; min-width: 0;
    font-size: 14px; line-height: 20px; font-weight: 650; padding: 0;
    color: inherit; text-align: left; display: block; user-select: none;
  }
  dl { margin: 0; display: grid; gap: 2px; }
  .row { display: flex; gap: 8px; }
  dt { color: var(--muted); min-width: 52px; }
  dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  dd a { color: var(--link); text-decoration: underline; text-decoration-color: var(--link-line); }
  dd a:hover { text-decoration-color: var(--link); }
  .kids { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 4px; }
  .kid {
    border: 1px solid var(--kid-border); background: var(--kid-bg); border-radius: 6px;
    font-size: 11px; padding: 2px 8px;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .kid:hover { background: var(--accent-soft); border-color: var(--accent); color: var(--link); }
  .kid:active { background: rgba(79, 124, 255, 0.24); border-color: var(--link); color: var(--link); }
  /* action row: fills the inner width of the card and sits on the card edge it opens from */
  .nodepanel {
    display: flex; height: 34px; margin: 10px -12px -10px; background: inherit;
    border-style: solid; border-color: inherit; border-width: 1px 0 0 0; overflow: hidden;
    font-size: 12px; cursor: default;
  }
  .card.flap-above .nodepanel { margin: -10px -12px 10px; border-width: 0 0 1px 0; }
  .card.flap-below .nodepanel { border-radius: 0 0 9px 9px; }
  .card.flap-above .nodepanel { border-radius: 9px 9px 0 0; }
  .nodepanel button {
    flex: 1; min-width: 0; border: none; background: none; border-radius: 0; color: var(--text);
    display: flex; align-items: center; justify-content: center; white-space: nowrap; font-size: inherit; padding: 0;
  }
  .nodepanel button + button { border-left-style: solid; border-left-width: 1px; border-left-color: inherit; }
  .nodepanel button:hover { background: var(--hover-bg); color: var(--link); }
  .nodepanel button.danger:hover { background: var(--danger-soft); color: var(--danger-text); }
  .card :global(.conn) {
    opacity: 0; width: 12px; height: 12px; background: #4f7cff; border: 2px solid var(--panel);
  }
  .card:hover :global(.conn), .card.selected :global(.conn) { opacity: 1; }
  .card.flap-above :global(.conn), .card.flap-below :global(.conn) {
    visibility: hidden; pointer-events: none;
  }
  .card :global(.conn.side) { width: 10px; height: 10px; }
</style>
