<script lang="ts">
  import { BaseEdge, EdgeLabel, getSmoothStepPath } from '@xyflow/svelte';

  let {
    id,
    sourceX,
    sourceY,
    targetX,
    targetY,
    sourcePosition,
    targetPosition,
    label,
    data,
    selected,
    style,
    markerStart,
    markerEnd,
    interactionWidth,
  }: any = $props();

  let editing = $derived(Boolean((data as any)?.editing));
  let locked = $derived(Boolean((data as any)?.locked));
  let text = $derived(typeof label === 'string' ? label : '');

  let [path, labelX, labelY] = $derived(
    getSmoothStepPath({ sourceX, sourceY, targetX, targetY, sourcePosition, targetPosition }),
  );

  let inputEl: HTMLInputElement | null = $state(null);
  $effect(() => {
    if (editing && inputEl) {
      inputEl.focus();
      inputEl.select();
    }
  });

  function startEdit() {
    window.dispatchEvent(new CustomEvent('edit-edge', { detail: id }));
  }

  function commit() {
    window.dispatchEvent(new CustomEvent('commit-edge', { detail: { id, value: inputEl?.value ?? text } }));
  }

  function cancel() {
    window.dispatchEvent(new CustomEvent('cancel-edge', { detail: id }));
  }
</script>

<BaseEdge {id} {path} {style} {markerStart} {markerEnd} {interactionWidth} />
{#if editing && !locked}
  <EdgeLabel x={labelX} y={labelY} transparent>
    <input
      class="elabel-edit"
      bind:this={inputEl}
      value={text}
      onblur={commit}
      onkeydown={(e) => {
        if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
        if (e.key === 'Escape') {
          e.stopPropagation();
          cancel();
        }
      }}
    />
  </EdgeLabel>
{:else if text}
  <EdgeLabel x={labelX} y={labelY} selectEdgeOnClick>
    {#if locked}
      <span class="elabel">{text}</span>
    {:else}
      <button class="elabel" title="Click to rename" onclick={startEdit}>{text}</button>
    {/if}
  </EdgeLabel>
{:else if selected && !locked}
  <EdgeLabel x={labelX} y={labelY} selectEdgeOnClick>
    <button class="elabel ghost" title="Add label" onclick={startEdit}>+ label</button>
  </EdgeLabel>
{/if}

<style>
  /* Label button and rename input share one box, so the text does not move on click. */
  .elabel,
  .elabel-edit {
    box-sizing: border-box;
    height: 24px;
    margin: 0;
    padding: 0 8px;
    font-size: 12px;
    line-height: 22px;
    border: 1px solid transparent;
    border-radius: 6px;
    background: transparent;
    color: inherit;
  }
  .elabel:hover {
    background: var(--accent-soft);
    color: var(--link);
  }
  .elabel.ghost {
    color: #98a2b3;
    border-style: dashed;
    border-color: var(--border);
  }
  .elabel-edit {
    width: 140px;
    background: var(--panel);
    border-color: var(--accent, #4f7cff);
    color: var(--text);
  }
  .elabel-edit:focus {
    outline: none;
  }
</style>
