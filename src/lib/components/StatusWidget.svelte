<script lang="ts">
  import { tauriInvoke } from '$lib/utils/tauri';

  interface Status {
    battery: { present: boolean; percent: number; charging: boolean };
    wifi: { connected: boolean; is_wifi: boolean; name: string; bars: number };
    volume: { available: boolean; level: number; muted: boolean };
  }

  let now = $state(new Date());
  let status = $state<Status | null>(null);
  let dragging = false;
  let volume = $state(0);

  $effect(() => {
    const id = setInterval(() => { now = new Date(); }, 1000);
    return () => clearInterval(id);
  });

  async function refresh() {
    try {
      status = await tauriInvoke<Status>('get_status');
      if (!dragging) volume = Math.round(status.volume.level * 100);
    } catch {
      status = null;
    }
  }

  $effect(() => {
    refresh();
    const id = setInterval(refresh, 2000);
    return () => clearInterval(id);
  });

  const time = $derived(now.toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' }));
  const date = $derived(
    now.toLocaleDateString([], { weekday: 'long', day: 'numeric', month: 'long', year: 'numeric' })
  );

  async function onVolume(e: Event) {
    dragging = true;
    volume = Number((e.target as HTMLInputElement).value);
    await tauriInvoke('set_volume', { level: volume / 100 }).catch(() => {});
  }

  async function toggleMute() {
    if (!status) return;
    const muted = !status.volume.muted;
    status.volume.muted = muted;
    await tauriInvoke('set_muted', { muted }).catch(() => {});
  }

  const batteryColor = $derived(
    !status || status.battery.charging ? '#30d158' : status.battery.percent <= 20 ? '#ff453a' : '#fff'
  );
</script>

<div class="status">
  <div class="clock">
    <span class="time">{time}</span>
    <span class="date">{date}</span>
  </div>

  <div class="rows">
    {#if status?.battery.present}
      <div class="row">
        <span class="label">Battery</span>
        <div class="bar"><div class="fill" style="width: {status.battery.percent}%; background: {batteryColor}"></div></div>
        <span class="value">{Math.round(status.battery.percent)}%{status.battery.charging ? ' ⚡' : ''}</span>
      </div>
    {/if}

    <div class="row">
      <span class="label">Wi-Fi</span>
      <span class="grow">
        {#if !status}…
        {:else if !status.wifi.connected}Not connected
        {:else if status.wifi.is_wifi}{status.wifi.name}
        {:else}Wired
        {/if}
      </span>
      {#if status?.wifi.connected && status.wifi.is_wifi}
        <span class="bars" aria-label="{status.wifi.bars} of 5 bars">
          {#each [1, 2, 3, 4, 5] as n}<i class:on={n <= status.wifi.bars} style="height: {4 + n * 2}px"></i>{/each}
        </span>
      {/if}
    </div>

    {#if status?.volume.available}
      <div class="row">
        <button class="mute" onclick={toggleMute} title={status.volume.muted ? 'Unmute' : 'Mute'}>
          {status.volume.muted || volume === 0 ? '🔇' : volume < 50 ? '🔉' : '🔊'}
        </button>
        <input
          class="slider"
          type="range"
          min="0"
          max="100"
          value={volume}
          oninput={onVolume}
          onchange={() => { dragging = false; }}
          style="--fill: {volume}%"
        />
        <span class="value">{volume}%</span>
      </div>
    {/if}
  </div>
</div>

<style>
  .status {
    height: 100%;
    display: flex;
    flex-direction: column;
    gap: 14px;
    color: #fff;
  }

  .clock {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding-top: 4px;
  }
  .time { font-size: 38px; font-weight: 600; letter-spacing: -1px; font-variant-numeric: tabular-nums; }
  .date { font-size: 12px; color: rgba(255, 255, 255, 0.55); }

  .rows { display: flex; flex-direction: column; gap: 10px; }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12px;
    min-height: 26px;
  }
  .label { width: 50px; color: rgba(255, 255, 255, 0.55); }
  .grow { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .value { width: 48px; text-align: right; font-variant-numeric: tabular-nums; }

  .bar { flex: 1; height: 8px; border-radius: 4px; background: rgba(255, 255, 255, 0.14); overflow: hidden; }
  .fill { height: 100%; border-radius: 4px; transition: width 0.4s ease; }

  .bars { display: flex; align-items: flex-end; gap: 2px; height: 14px; }
  .bars i { width: 3px; border-radius: 1px; background: rgba(255, 255, 255, 0.22); }
  .bars i.on { background: #fff; }

  .mute {
    width: 50px;
    border: none;
    background: transparent;
    font-size: 15px;
    cursor: pointer;
    padding: 0;
    text-align: left;
  }

  .slider {
    flex: 1;
    -webkit-appearance: none;
    appearance: none;
    height: 8px;
    border-radius: 4px;
    background: linear-gradient(to right, #fff var(--fill), rgba(255, 255, 255, 0.14) var(--fill));
    outline: none;
    cursor: pointer;
  }
  .slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 1px 3px rgba(0, 0, 0, 0.5);
  }
</style>
