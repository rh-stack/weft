<script lang="ts">
  import { NodeResizer } from '@xyflow/svelte';
  export let data: { title: string };
  export let selected: boolean = false;
</script>

<div class="area {selected ? 'selected' : ''}">
  <NodeResizer isVisible={selected} minWidth={240} minHeight={160} />
  <div class="label">{data.title}</div>
  <div class="hint">{selected ? 'drag sides or corners to resize' : ''}</div>
</div>

<style>
  .area {
    width: 100%;
    height: 100%;
    min-width: 240px;
    min-height: 160px;
    background: rgba(148, 163, 184, 0.08);
    border: 1px dashed #b6bfcc;
    border-radius: 12px;
    padding: 8px 10px;
    box-sizing: border-box;
    cursor: default;
  }
  .area.selected { border-color: #4f7cff; border-style: solid; }
  .label { font-size: 12px; font-weight: 650; color: #475569; }
  .hint { font-size: 11px; color: #94a3b8; }
</style>
