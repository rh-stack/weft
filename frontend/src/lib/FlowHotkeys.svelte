<script lang="ts">
  import { useStore } from '@xyflow/svelte';

  // Keyboard shortcuts for the canvas: F fits the view, L toggles interactivity.
  // Must render inside <SvelteFlow> to see the shared store.
  let { disabled = false }: { disabled?: boolean } = $props();
  const store = useStore();

  function isTyping(t: HTMLElement | null): boolean {
    return !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
  }

  function toggleInteractivity() {
    const interactive = !(store.nodesDraggable || store.nodesConnectable || store.elementsSelectable);
    store.nodesDraggable = interactive;
    store.nodesConnectable = interactive;
    store.elementsSelectable = interactive;
  }

  function onKey(e: KeyboardEvent) {
    if (disabled || e.ctrlKey || e.metaKey || e.altKey) return;
    if (isTyping(document.activeElement as HTMLElement | null)) return;
    const key = e.key.toLowerCase();
    if (key === 'f') {
      e.preventDefault();
      store.fitView({ padding: 0.3 });
    } else if (key === 'l') {
      e.preventDefault();
      toggleInteractivity();
    }
  }
</script>

<svelte:window onkeydown={onKey} />
