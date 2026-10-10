<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { Crepe, CrepeFeature } from '@milkdown/crepe';
  import '@milkdown/crepe/theme/common/style.css';
  import '@milkdown/crepe/theme/classic.css';
  import { uploadAsset } from './api';

  export let value: string;
  export let onChange: (md: string) => void;

  // Crepe's image node needs a string caption. Plain `![alt](src)`
  // throws at parse and drops the image, so titles get added on load.
  function withCaptions(md: string): string {
    return (md ?? '').replace(/!\[([^\]]*)\]\((\S+?)\)/g, (_m, alt, src) => `![${alt}](${src} "${alt || 'image'}")`);
  }

  // Files stay self-sufficient: titles go in before save, and the
  // serializer's numeric alt text (image ratio) becomes the caption.
  function sanitizeSave(md: string): string {
    return withCaptions(md).replace(/!\[([\d.]+)\]\((\S+?)(?: "([^"]*)")?\)/g, (_m, _alt, src, cap) =>
      `![${cap || 'image'}](${src} "${cap || 'image'}")`,
    );
  }

  let root: HTMLDivElement;
  let crepe: Crepe | null = null;
  let alive = true;

  onMount(() => {
    (async () => {
      const c = new Crepe({
        root,
        defaultValue: withCaptions(value),
        featureConfigs: {
          [CrepeFeature.Cursor]: {
            color: '#4f7cff',
            width: 2,
          },
          [CrepeFeature.ImageBlock]: {
            onUpload: async (file: File) => {
              const safe = `${Date.now()}-${file.name.replace(/[^a-zA-Z0-9._-]/g, '_')}`;
              const name = await uploadAsset(safe, file);
              // Files are served under /uploads (the /assets prefix
              // belongs to the frontend bundle).
              return `/uploads/${name}`;
            },
          },
        },
      });
      await c.create();
      if (!alive) {
        await c.destroy();
        return;
      }
      crepe = c;
      root.querySelector('[contenteditable]')?.setAttribute('spellcheck', 'false');
      c.on((li) => {
        li.markdownUpdated((_ctx, md: string) => {
          onChange(sanitizeSave(md));
        });
      });
    })();
    return () => {
      alive = false;
    };
  });

  onDestroy(() => {
    alive = false;
    if (crepe) {
      crepe.destroy();
      crepe = null;
    }
  });
</script>

<div class="milk" bind:this={root}></div>

<style>
  .milk :global(.milkdown) {
    font: 14px/1.65 system-ui, sans-serif;
    color: var(--text);
  }
  /* Crepe dark palette, scoped to the dark theme. The stock classic-dark
     sheet cannot load next to the light one, so its colors are copied here. */
  :global(:root[data-theme='dark']) .milk :global(.milkdown) {
    --crepe-color-background: #1f1b16;
    --crepe-color-on-background: #eae1d9;
    --crepe-color-surface: #18120b;
    --crepe-color-surface-low: #201b13;
    --crepe-color-on-surface: #ede0d4;
    --crepe-color-on-surface-variant: #d3c4b4;
    --crepe-color-outline: #9c8f80;
    --crepe-color-primary: #f4bd6f;
    --crepe-color-secondary: #56442a;
    --crepe-color-on-secondary: #fbdebc;
    --crepe-color-inverse: #ede0d4;
    --crepe-color-on-inverse: #362f27;
    --crepe-color-inline-code: #ffb4ab;
    --crepe-color-error: #ffb4ab;
    --crepe-color-hover: #251f17;
    --crepe-color-selected: #3b342b;
    --crepe-color-inline-area: #3f3830;
  }
  .milk :global(.milkdown) {
    background: transparent;
  }
  .milk :global(.milkdown .ProseMirror) {
    padding: 4px 2px;
    min-height: 280px;
    outline: none;
    caret-color: #4f7cff;
    background: transparent;
  }
  .milk :global(.milkdown h1),
  .milk :global(.milkdown h2),
  .milk :global(.milkdown h3) {
    font-weight: 650;
    letter-spacing: -0.01em;
  }
  .milk :global(.milkdown img) {
    border-radius: 8px;
    border: 1px solid var(--img-border);
  }
  .milk :global(.milkdown code) {
    font-family: ui-monospace, Consolas, monospace;
  }
</style>
