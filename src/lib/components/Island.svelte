<script lang="ts">
  import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';
  import { activeWidget, islandState, isHovered, type WidgetType } from '$lib/stores/island';
  import { tauriInvoke } from '$lib/utils/tauri';
  import MediaWidget from './MediaWidget.svelte';
  import SystemWidget from './SystemWidget.svelte';
  import WeatherWidget from './WeatherWidget.svelte';
  import NotificationWidget from './NotificationWidget.svelte';
  import TimerWidget from './TimerWidget.svelte';
  import ClipboardWidget from './ClipboardWidget.svelte';

  type Mode = 'idle' | 'media' | 'timer' | 'expanded' | 'wide';

  // Target size of the pill per mode (logical px).
  const SIZES: Record<Mode, { w: number; h: number }> = {
    idle: { w: 126, h: 34 },
    media: { w: 230, h: 38 },
    timer: { w: 200, h: 38 },
    expanded: { w: 380, h: 280 },
    wide: { w: 420, h: 200 },
  };
  const WINDOW_WIDTH = 440;
  const WINDOW_TOP_GAP = 8;

  let currentWidget: WidgetType = $state('system');
  let expandedMode = $state<'expanded' | 'wide' | null>(null);
  let hovered = $state(false);
  let timerRemaining = $state(0);
  let mediaPlaying = $state(false);
  let collapseTimeout: ReturnType<typeof setTimeout> | undefined;

  // Live activities shown while collapsed, in priority order.
  const mode = $derived<Mode>(
    expandedMode ?? (timerRemaining > 0 ? 'timer' : mediaPlaying ? 'media' : 'idle')
  );
  const target = $derived(SIZES[mode]);
  const isOpen = $derived(expandedMode !== null);

  $effect(() => { activeWidget.set(currentWidget); });
  $effect(() => { islandState.set(isOpen ? (expandedMode as 'expanded' | 'wide') : 'compact'); });
  $effect(() => { isHovered.set(hovered); });

  // Poll the live activities (silently ignore backend errors / no player).
  $effect(() => {
    async function poll() {
      try {
        const t = await tauriInvoke<{ active: boolean; remaining_seconds: number }>('get_timer_status');
        timerRemaining = t.active ? t.remaining_seconds : 0;
      } catch { timerRemaining = 0; }
      try {
        const m = await tauriInvoke<{ playing: boolean } | null>('get_media_status');
        mediaPlaying = !!m?.playing;
      } catch { mediaPlaying = false; }
    }
    poll();
    const id = setInterval(poll, 1000);
    return () => clearInterval(id);
  });

  // Spring physics: width, height and corner radius chase the target.
  let width = $state(SIZES.idle.w);
  let height = $state(SIZES.idle.h);
  const radius = $derived(Math.min(height / 2, 38));

  $effect(() => {
    const goalW = target.w;
    const goalH = target.h;
    let vw = 0;
    let vh = 0;
    let last = performance.now();
    let frame = requestAnimationFrame(step);
    const STIFFNESS = 220;
    const DAMPING = 17;

    function step(now: number) {
      const dt = Math.min((now - last) / 1000, 1 / 30);
      last = now;
      vw += (-STIFFNESS * (width - goalW) - DAMPING * vw) * dt;
      vh += (-STIFFNESS * (height - goalH) - DAMPING * vh) * dt;
      width += vw * dt;
      height += vh * dt;
      const settled =
        Math.abs(width - goalW) < 0.1 && Math.abs(height - goalH) < 0.1 &&
        Math.abs(vw) < 0.1 && Math.abs(vh) < 0.1;
      if (settled) {
        width = goalW;
        height = goalH;
        return;
      }
      frame = requestAnimationFrame(step);
    }
    return () => cancelAnimationFrame(frame);
  });

  // Keep the transparent OS window just big enough so it never blocks clicks
  // on the apps underneath. Grow immediately, shrink once the pill has settled.
  let windowHeight = 0;
  $effect(() => {
    const needed = Math.ceil(Math.max(height, target.h)) + WINDOW_TOP_GAP * 2 + 8;
    const settled = Math.abs(height - target.h) < 1;
    if (needed > windowHeight || (settled && needed !== windowHeight)) {
      windowHeight = needed;
      getCurrentWindow()
        .setSize(new LogicalSize(WINDOW_WIDTH, needed))
        .catch(() => {});
    }
  });

  function open(widget?: WidgetType) {
    if (widget) currentWidget = widget;
    expandedMode = currentWidget === 'media' ? 'wide' : 'expanded';
  }

  function collapse() {
    expandedMode = null;
  }

  function selectWidget(w: WidgetType) {
    currentWidget = w;
    expandedMode = w === 'media' ? 'wide' : 'expanded';
  }

  function handleEnter() {
    hovered = true;
    clearTimeout(collapseTimeout);
  }

  function handleLeave() {
    hovered = false;
    if (isOpen) collapseTimeout = setTimeout(collapse, 2000);
  }

  function fmt(sec: number) {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  const widgets: { type: WidgetType; icon: string; label: string }[] = [
    { type: 'system', icon: '⚡', label: 'System' },
    { type: 'media', icon: '🎵', label: 'Media' },
    { type: 'weather', icon: '🌸', label: 'Weather' },
    { type: 'notifications', icon: '🔔', label: 'Alerts' },
    { type: 'timer', icon: '⏱', label: 'Timer' },
    { type: 'clipboard', icon: '📋', label: 'Clips' },
  ];
</script>

<div
  class="island"
  class:hovered
  role="button"
  tabindex="0"
  style="width: {width}px; height: {height}px; border-radius: {radius}px;"
  onmouseenter={handleEnter}
  onmouseleave={handleLeave}
  onclick={() => { if (!isOpen) open(mode === 'timer' ? 'timer' : mode === 'media' ? 'media' : undefined); }}
  onkeydown={(e) => { if (e.key === 'Enter' && !isOpen) open(); }}
>
  {#if mode === 'media'}
    <div class="activity">
      <span class="note">♪</span>
      <div class="wave" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
    </div>
  {:else if mode === 'timer'}
    <div class="activity">
      <span class="note">⏱</span>
      <span class="timer-text">{fmt(timerRemaining)}</span>
    </div>
  {:else if isOpen}
    <div class="expanded-content">
      <div class="tab-bar">
        {#each widgets as w}
          <button
            class="tab-btn"
            class:active={currentWidget === w.type}
            onclick={(e) => { e.stopPropagation(); selectWidget(w.type); }}
          >
            <span>{w.icon}</span>
            <span class="tab-label">{w.label}</span>
          </button>
        {/each}
        <button class="collapse-btn" onclick={(e) => { e.stopPropagation(); collapse(); }} title="Collapse">
          <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
            <path d="M3 5L7 9L11 5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
        </button>
      </div>
      <div class="widget-area">
        {#if currentWidget === 'media'}<MediaWidget />
        {:else if currentWidget === 'system'}<SystemWidget />
        {:else if currentWidget === 'weather'}<WeatherWidget />
        {:else if currentWidget === 'notifications'}<NotificationWidget />
        {:else if currentWidget === 'timer'}<TimerWidget />
        {:else if currentWidget === 'clipboard'}<ClipboardWidget />
        {/if}
      </div>
    </div>
  {/if}
</div>

<style>
  .island {
    position: fixed;
    top: 8px;
    left: 50%;
    transform: translateX(-50%);
    background: #000;
    overflow: hidden;
    cursor: default;
    z-index: 9999;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35);
    user-select: none;
  }

  .island:not(:has(.expanded-content)) { cursor: pointer; }

  .activity {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 100%;
    padding: 0 16px;
    color: #fff;
    animation: fade-in 0.25s ease 0.08s both;
  }

  .note { font-size: 14px; opacity: 0.9; }

  .timer-text {
    font-size: 14px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: #ff9f0a;
  }

  .wave { display: flex; align-items: center; gap: 3px; height: 16px; }
  .wave i {
    width: 3px;
    height: 100%;
    border-radius: 2px;
    background: #30d158;
    transform-origin: center;
    animation: bounce 0.9s ease-in-out infinite;
  }
  .wave i:nth-child(2) { animation-delay: 0.15s; }
  .wave i:nth-child(3) { animation-delay: 0.3s; }
  .wave i:nth-child(4) { animation-delay: 0.45s; }

  @keyframes bounce {
    0%, 100% { transform: scaleY(0.25); }
    50% { transform: scaleY(1); }
  }

  .expanded-content {
    display: flex;
    flex-direction: column;
    height: 100%;
    animation: fade-in 0.3s ease 0.1s both;
  }

  .tab-bar {
    display: flex;
    align-items: center;
    gap: 2px;
    padding: 8px 10px 6px;
    flex-shrink: 0;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 3px;
    padding: 5px 9px;
    border: none;
    border-radius: 14px;
    background: transparent;
    color: rgba(255, 255, 255, 0.5);
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease;
    white-space: nowrap;
  }
  .tab-btn:hover { background: rgba(255, 255, 255, 0.1); color: #fff; }
  .tab-btn.active { background: rgba(255, 255, 255, 0.16); color: #fff; }
  .tab-label { font-size: 10px; font-weight: 500; }

  .collapse-btn {
    margin-left: auto;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: none;
    background: rgba(255, 255, 255, 0.12);
    color: rgba(255, 255, 255, 0.7);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    padding: 0;
  }
  .collapse-btn:hover { background: rgba(255, 255, 255, 0.22); color: #fff; }

  .widget-area {
    flex: 1;
    overflow: hidden;
    padding: 4px 14px 12px;
    min-height: 0;
  }

  @keyframes fade-in {
    from { opacity: 0; }
    to { opacity: 1; }
  }
</style>
