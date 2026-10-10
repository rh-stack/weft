<script lang="ts">
  import { useStore } from '@xyflow/svelte';

  // Mirrors the Controls lock button into the app's `locked` flag.
  // Must render inside <SvelteFlow> to see the shared store.
  let { onlock }: { onlock: (v: boolean) => void } = $props();
  const store = useStore();
  let prev: boolean | null = null;
  $effect(() => {
    const v = !(store.nodesDraggable || store.nodesConnectable || store.elementsSelectable);
    if (v !== prev) {
      prev = v;
      onlock(v);
    }
  });
</script>
