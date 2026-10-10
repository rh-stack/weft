<script lang="ts">
  import { Handle, Position } from '@xyflow/svelte';
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

  function isLink(v: unknown): boolean {
    return /^https?:\/\//i.test(String(v ?? ''));
  }
</script>

<div
  class="card {selected ? 'selected' : ''} {data.dimmed ? 'dimmed' : ''}"
  style="background: {data.color ?? '#ffffff'}"
>
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
</div>

<style>
  .card {
    min-width: 220px;
    max-width: 300px;
    background: #fff;
    border: 1px solid #d8dae0;
    border-radius: 10px;
    padding: 10px 12px;
    font-size: 12px;
    color: #1c2333;
    box-shadow: 0 1px 2px rgba(16, 24, 40, 0.06);
    cursor: default;
  }
  .card :global(button) { cursor: default; }
  .card.selected { border-color: #4f7cff; }
  .card.dimmed { opacity: 0.25; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 8px; margin: 4px 0 6px; }
  .type {
    flex: none;
    font-size: 10px; line-height: 18px; white-space: nowrap; text-transform: uppercase; letter-spacing: 0.06em;
    color: #667085; background: #f2f4f7; border-radius: 4px; padding: 1px 6px;
  }
  .titletext {
    flex: 1; min-width: 0;
    font-size: 14px; line-height: 20px; font-weight: 650; padding: 0;
    color: inherit; text-align: left; display: block; user-select: none;
  }
  dl { margin: 0; display: grid; gap: 2px; }
  .row { display: flex; gap: 8px; }
  dt { color: #667085; min-width: 52px; }
  dd { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  dd a { color: #2f54c9; text-decoration: underline; text-decoration-color: rgba(47, 84, 201, 0.35); }
  dd a:hover { text-decoration-color: #2f54c9; }
  .kids { margin-top: 8px; display: flex; flex-wrap: wrap; gap: 4px; }
  .kid {
    border: 1px solid rgba(16, 24, 40, 0.14); background: rgba(255, 255, 255, 0.7); border-radius: 6px;
    font-size: 11px; padding: 2px 8px;
    transition: background 0.12s, border-color 0.12s, color 0.12s;
  }
  .kid:hover { background: rgba(79, 124, 255, 0.14); border-color: #4f7cff; color: #2f54c9; }
  .kid:active { background: rgba(79, 124, 255, 0.24); border-color: #2f54c9; color: #2f54c9; }
  /* connection handles appear on hover or selection */
  .card :global(.conn) {
    opacity: 0; width: 12px; height: 12px; background: #4f7cff; border: 2px solid #fff;
  }
  .card:hover :global(.conn), .card.selected :global(.conn) { opacity: 1; }
  .card :global(.conn.side) { width: 10px; height: 10px; }
</style>
