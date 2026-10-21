<script lang="ts">
  import { untrack } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { getCurrentWindow, currentMonitor } from '@tauri-apps/api/window';
  import { activeWidget, islandState, isHovered, type WidgetType } from '$lib/stores/island';
  import { tauriInvoke } from '$lib/utils/tauri';
  import AppsWidget from './AppsWidget.svelte';
  import StatusWidget from './StatusWidget.svelte';
  import MediaWidget from './MediaWidget.svelte';
  import SystemWidget from './SystemWidget.svelte';
  import WeatherWidget from './WeatherWidget.svelte';
  import NotificationWidget from './NotificationWidget.svelte';
  import TimerWidget from './TimerWidget.svelte';
  import ClipboardWidget from './ClipboardWidget.svelte';

  type Mode = 'idle' | 'media' | 'timer' | 'expanded' | 'wide';
  type Edge = 'top' | 'left' | 'right';
  type Rect = { x: number; y: number; w: number; h: number };

  // Pill size per mode in horizontal orientation (logical px). The collapsed
  // modes are swapped to be tall instead of wide when docked to a side edge.
  const BASE: Record<Mode, { w: number; h: number }> = {
    idle: { w: 126, h: 34 },
    media: { w: 230, h: 38 },
    timer: { w: 200, h: 38 },
    expanded: { w: 380, h: 280 },
    wide: { w: 420, h: 200 },
  };
  const PAD = 4; // transparent margin kept around the pill inside the OS window
  const MARGIN = 6; // gap between the pill and the screen edge
  const EDGE_ZONE = 100; // closer than this to a side edge => vertical pill
  const DRAG_THRESHOLD = 5;
  const STORAGE_KEY = 'island-anchor';

  function sizeFor(mode: Mode, vertical: boolean) {
    const s = BASE[mode];
    return vertical && (mode === 'idle' || mode === 'media' || mode === 'timer')
      ? { w: s.h, h: s.w }
      : s;
  }

  const clamp = (v: number, lo: number, hi: number) => Math.min(Math.max(v, lo), Math.max(lo, hi));

  let currentWidget: WidgetType = $state('apps');
  let expandedMode = $state<'expanded' | 'wide' | null>(null);
  let hovered = $state(false);
  let timerRemaining = $state(0);
  let mediaPlaying = $state(false);

  // Where the island lives. For 'top' the anchor is the top-centre of the pill,
  // for 'left'/'right' it is the centre of the edge the pill is attached to.
  let edge = $state<Edge>('top');
  let anchor = $state({ x: 0, y: 0 });
  let screen = $state({ x: 0, y: 0, w: 1920, h: 1080 });
  let ready = $state(false);
  let winRect = $state<Rect>({ x: 0, y: 0, w: BASE.idle.w + PAD * 2, h: BASE.idle.h + PAD * 2 });

  const vertical = $derived(edge !== 'top');

  // Live activities shown while collapsed, in priority order.
  const mode = $derived<Mode>(
    expandedMode ?? (timerRemaining > 0 ? 'timer' : mediaPlaying ? 'media' : 'idle')
  );
  const target = $derived(sizeFor(mode, vertical));
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

  // Spring physics: width and height chase the target.
  let width = $state(BASE.idle.w);
  let height = $state(BASE.idle.h);
  const radius = $derived(Math.min(Math.min(width, height) / 2, 38));

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

  // ── OS window geometry ────────────────────────────────────────────────────
  // The transparent window is kept exactly as big as the pill (plus PAD) so it
  // never blocks clicks or typing on the apps around it. It grows immediately
  // and shrinks once the spring has settled.
  let appliedInner = { w: 0, h: 0 };
  const queue: { next: Rect | null; busy: boolean } = { next: null, busy: false };

  async function pushRect(rect: Rect) {
    queue.next = rect;
    if (queue.busy) return;
    queue.busy = true;
    try {
      while (queue.next) {
        const r = queue.next;
        queue.next = null;
        await invoke('set_window_rect', { x: r.x, y: r.y, w: r.w, h: r.h });
      }
    } catch {
      // window API unavailable (e.g. plain browser) — nothing to resize
    } finally {
      queue.busy = false;
    }
  }

  $effect(() => {
    if (!ready) return;
    const settled = Math.abs(width - target.w) < 1 && Math.abs(height - target.h) < 1;
    const innerW = Math.ceil(settled ? target.w : Math.max(target.w, appliedInner.w));
    const innerH = Math.ceil(settled ? target.h : Math.max(target.h, appliedInner.h));
    const W = innerW + PAD * 2;
    const H = innerH + PAD * 2;

    let x: number;
    let y: number;
    if (edge === 'top') {
      x = anchor.x - W / 2;
      y = anchor.y - PAD;
    } else if (edge === 'left') {
      x = anchor.x - PAD;
      y = anchor.y - H / 2;
    } else {
      x = anchor.x + PAD - W;
      y = anchor.y - H / 2;
    }
    x = Math.round(clamp(x, screen.x, screen.x + screen.w - W));
    y = Math.round(clamp(y, screen.y, screen.y + screen.h - H));

    const prev = untrack(() => winRect);
    if (prev.x === x && prev.y === y && prev.w === W && prev.h === H) return;
    appliedInner = { w: innerW, h: innerH };
    winRect = { x, y, w: W, h: H };
    pushRect({ x, y, w: W, h: H });
  });

  // Pill position inside the window (stays glued to the anchor while it grows).
  const pillLeft = $derived(
    clamp(
      (edge === 'top'
        ? anchor.x - width / 2
        : edge === 'left'
          ? anchor.x
          : anchor.x - width) - winRect.x,
      PAD,
      winRect.w - PAD - width
    )
  );
  const pillTop = $derived(
    clamp(
      (edge === 'top' ? anchor.y : anchor.y - height / 2) - winRect.y,
      PAD,
      winRect.h - PAD - height
    )
  );

  async function refreshScreen() {
    const m = await currentMonitor();
    if (!m) return;
    const s = m.scaleFactor;
    screen = { x: m.position.x / s, y: m.position.y / s, w: m.size.width / s, h: m.size.height / s };
  }

  function saveAnchor() {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ edge, x: anchor.x, y: anchor.y }));
    } catch {}
  }

  function loadAnchor(): { edge: Edge; x: number; y: number } | null {
    try {
      const raw = localStorage.getItem(STORAGE_KEY);
      if (!raw) return null;
      const a = JSON.parse(raw);
      if (['top', 'left', 'right'].includes(a.edge) && Number.isFinite(a.x) && Number.isFinite(a.y)) return a;
    } catch {}
    return null;
  }

  $effect(() => {
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    (async () => {
      try {
        await refreshScreen();
      } catch {}
      const saved = loadAnchor();
      if (saved) {
        edge = saved.edge;
        anchor = {
          x: clamp(saved.x, screen.x, screen.x + screen.w),
          y: clamp(saved.y, screen.y, screen.y + screen.h),
        };
      } else {
        edge = 'top';
        anchor = { x: screen.x + screen.w / 2, y: screen.y + MARGIN + 2 };
      }
      ready = true;
      try {
        const off = await getCurrentWindow().onMoved(onMoved);
        if (cancelled) off();
        else unlisten = off;
      } catch {}
    })();
    return () => {
      cancelled = true;
      unlisten?.();
    };
  });

  // ── Dragging ──────────────────────────────────────────────────────────────
  let downPos: { x: number; y: number } | null = null;
  let dragStarted = false;
  let userDragging = false;
  let moveTimer: ReturnType<typeof setTimeout> | undefined;

  function onPointerDown(e: PointerEvent) {
    dragStarted = false;
    if (isOpen || e.button !== 0) return;
    downPos = { x: e.screenX, y: e.screenY };
  }

  function onPointerMove(e: PointerEvent) {
    if (!downPos || isOpen || dragStarted) return;
    if (Math.hypot(e.screenX - downPos.x, e.screenY - downPos.y) > DRAG_THRESHOLD) {
      dragStarted = true;
      userDragging = true;
      downPos = null;
      getCurrentWindow().startDragging().catch(() => { userDragging = false; });
    }
  }

  function onPointerUp() {
    downPos = null;
  }

  function onMoved() {
    if (!userDragging) return; // ignore moves caused by our own setPosition
    clearTimeout(moveTimer);
    moveTimer = setTimeout(finishDrag, 250);
  }

  // After a drag: decide which edge the island belongs to and re-anchor it there.
  async function finishDrag() {
    try {
      const win = getCurrentWindow();
      const [pos, size, scale] = await Promise.all([win.outerPosition(), win.outerSize(), win.scaleFactor()]);
      await refreshScreen();

      const real = { x: pos.x / scale, y: pos.y / scale, w: size.width / scale, h: size.height / scale };
      const cx = real.x + real.w / 2;
      const cy = real.y + real.h / 2;
      winRect = real; // keep the pill visually still while the anchor changes
      appliedInner = { w: real.w - PAD * 2, h: real.h - PAD * 2 };

      let next: Edge = 'top';
      if (cx - screen.x < EDGE_ZONE) next = 'left';
      else if (screen.x + screen.w - cx < EDGE_ZONE) next = 'right';

      edge = next;
      anchor =
        next === 'top'
          ? { x: cx, y: Math.max(real.y + PAD, screen.y + MARGIN) }
          : next === 'left'
            ? { x: screen.x + MARGIN, y: cy }
            : { x: screen.x + screen.w - MARGIN, y: cy };
      saveAnchor();
    } catch {
      // keep current placement
    } finally {
      userDragging = false;
    }
  }

  // ── Open / close ──────────────────────────────────────────────────────────
  let leaveTimer: ReturnType<typeof setTimeout> | undefined;
  let openedAt = 0;

  function open(widget?: WidgetType) {
    openedAt = performance.now();
    if (widget) currentWidget = widget;
    expandedMode = currentWidget === 'media' ? 'wide' : 'expanded';
  }

  function collapse() {
    expandedMode = null;
  }

  function selectWidget(w: WidgetType) {
    openedAt = performance.now();
    currentWidget = w;
    expandedMode = w === 'media' ? 'wide' : 'expanded';
  }

  function handleClick() {
    if (dragStarted) {
      dragStarted = false;
      return;
    }
    if (!isOpen) open(mode === 'timer' ? 'timer' : mode === 'media' ? 'media' : 'apps');
  }

  function handleEnter() {
    hovered = true;
    clearTimeout(leaveTimer);
  }

  // The panel closes as soon as the cursor leaves the island. The short delay
  // (longer right after opening) only filters out the window resizing itself.
  function handleLeave() {
    hovered = false;
    downPos = null;
    if (!isOpen) return;
    clearTimeout(leaveTimer);
    const sinceOpen = performance.now() - openedAt;
    leaveTimer = setTimeout(() => { if (!hovered) collapse(); }, Math.max(120, 450 - sinceOpen));
  }

  function fmt(sec: number) {
    const m = Math.floor(sec / 60);
    const s = Math.floor(sec % 60);
    return `${m}:${s.toString().padStart(2, '0')}`;
  }

  const widgets: { type: WidgetType; icon: string; label: string }[] = [
    { type: 'apps', icon: '🪟', label: 'Apps' },
    { type: 'status', icon: '🔋', label: 'Status' },
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
  class:vertical
  class:open={isOpen}
  role="button"
  tabindex="0"
  style="left: {pillLeft}px; top: {pillTop}px; width: {width}px; height: {height}px; border-radius: {radius}px;"
  onmouseenter={handleEnter}
  onmouseleave={handleLeave}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onclick={handleClick}
  onkeydown={(e) => { if (e.key === 'Enter' && !isOpen) open('apps'); }}
>
  {#if mode === 'media'}
    <div class="activity" class:vertical>
      <span class="note">♪</span>
      <div class="wave" aria-hidden="true"><i></i><i></i><i></i><i></i></div>
    </div>
  {:else if mode === 'timer'}
    <div class="activity" class:vertical>
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
      </div>
      <div class="widget-area">
        {#if currentWidget === 'apps'}<AppsWidget />
        {:else if currentWidget === 'status'}<StatusWidget />
        {:else if currentWidget === 'media'}<MediaWidget />
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
    position: absolute;
    background: #000;
    overflow: hidden;
    cursor: pointer;
    z-index: 9999;
    box-shadow: 0 1px 4px rgba(0, 0, 0, 0.4);
    user-select: none;
    touch-action: none;
  }

  .island.open { cursor: default; }

  .activity {
    display: flex;
    align-items: center;
    justify-content: space-between;
    height: 100%;
    padding: 0 16px;
    color: #fff;
    animation: fade-in 0.25s ease 0.08s both;
  }

  .activity.vertical {
    flex-direction: column;
    padding: 16px 0;
  }

  .note { font-size: 14px; opacity: 0.9; }

  .timer-text {
    font-size: 14px;
    font-weight: 600;
    font-variant-numeric: tabular-nums;
    color: #ff9f0a;
  }

  .activity.vertical .timer-text { font-size: 12px; }

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
    padding: 5px 7px;
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
  .tab-btn:not(.active) .tab-label { display: none; }

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
