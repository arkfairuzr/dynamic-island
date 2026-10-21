<script lang="ts">
  import { tauriInvoke } from '$lib/utils/tauri';

  interface OpenWindow {
    id: number;
    title: string;
    app: string;
    active: boolean;
  }

  let windows = $state<OpenWindow[]>([]);
  let loaded = $state(false);

  async function refresh() {
    try {
      windows = await tauriInvoke<OpenWindow[]>('list_open_windows');
    } catch {
      windows = [];
    }
    loaded = true;
  }

  $effect(() => {
    refresh();
    const id = setInterval(refresh, 1500);
    return () => clearInterval(id);
  });

  async function focus(w: OpenWindow) {
    try {
      await tauriInvoke('focus_window', { id: w.id });
    } catch {}
    refresh();
  }

  // Stable colour per app so the same app always looks the same.
  function hue(name: string) {
    let h = 0;
    for (const c of name) h = (h * 31 + c.charCodeAt(0)) % 360;
    return h;
  }

  function initial(name: string) {
    return (name.trim()[0] ?? '?').toUpperCase();
  }
</script>

<div class="apps">
  {#if loaded && windows.length === 0}
    <p class="empty">No open apps</p>
  {:else}
    <div class="grid">
      {#each windows as w (w.id)}
        <button class="app" class:active={w.active} title={w.title} onclick={() => focus(w)}>
          <span class="icon" style="background: hsl({hue(w.app)} 55% 42%)">{initial(w.app)}</span>
          <span class="label">{w.app}</span>
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .apps {
    height: 100%;
    overflow-y: auto;
    scrollbar-width: none;
  }
  .apps::-webkit-scrollbar { display: none; }

  .grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }

  .app {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 5px;
    padding: 8px 4px 6px;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: rgba(255, 255, 255, 0.75);
    cursor: pointer;
    transition: background 0.15s ease;
    min-width: 0;
  }
  .app:hover { background: rgba(255, 255, 255, 0.1); }
  .app.active { background: rgba(255, 255, 255, 0.16); color: #fff; }

  .icon {
    width: 38px;
    height: 38px;
    border-radius: 10px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 17px;
    font-weight: 600;
    color: #fff;
  }

  .label {
    max-width: 100%;
    font-size: 10px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .empty {
    margin: 40px 0 0;
    text-align: center;
    font-size: 12px;
    color: rgba(255, 255, 255, 0.45);
  }
</style>
