<script lang="ts">
  import { onMount } from "svelte";

  // ════════════════════════════════════════════════════════════════════
  //  Signalume — a directional signal finder.
  //
  //  The whole app answers one question: "which way is the signal strongest,
  //  and where should I stand?" You slowly turn the phone; the strength meter
  //  rises and falls, and a single arrow always points toward the strongest
  //  direction. Turn until the arrow is up and it says you're facing it.
  //  "Save spot" pins where you're standing so you can compare places and walk
  //  back to the best one.
  //
  //  Honesty note: this is a prototype. The signal field is *simulated* (there
  //  is no real Wi-Fi scan here), but the interaction is real — it responds to
  //  the phone's actual compass. Nothing in the UI pretends to scan hardware.
  // ════════════════════════════════════════════════════════════════════

  // ── Angle helpers (degrees, shortest signed path) ─────────────────────
  const angDiff = (a: number, b: number) => ((b - a + 540) % 360) - 180;
  const circLerp = (a: number, target: number, k: number) =>
    (a + angDiff(a, target) * k + 360) % 360;
  const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n));

  // ── Settings (persisted) ──────────────────────────────────────────────
  type ThemeId = "beacon" | "tide" | "paper";
  type SmoothId = "responsive" | "balanced" | "steady";
  type Settings = {
    theme: ThemeId;
    smoothing: SmoothId;
    showDetails: boolean;   // reveal the technical readouts card
    haptics: boolean;       // vibrate when saving a reading
    buzzOnLock: boolean;    // vibrate the moment you face the strongest signal
    keepAwake: boolean;     // hold the screen on while hunting
  };
  let settings = $state<Settings>({
    theme: "beacon", smoothing: "balanced",
    showDetails: false, haptics: true, buzzOnLock: true, keepAwake: false,
  });
  let settingsOpen = $state(false);
  let ready = $state(false); // gate persistence until stored values are loaded

  const THEME_META: Record<ThemeId, { label: string; swatch: string }> = {
    beacon: { label: "Beacon", swatch: "#e0a04a" },
    tide:   { label: "Tide",   swatch: "#6fb3ad" },
    paper:  { label: "Paper",  swatch: "#e7e1d5" },
  };
  // Canvas can't read CSS vars cheaply per frame, so mirror the few colours it
  // needs as plain RGB triplets per theme: the accent (needle) and the "ink"
  // used for rings / crosshairs / labels (light on dark themes, dark on Paper).
  const THEME_ACCENT: Record<ThemeId, string> = {
    beacon: "224,160,74", tide: "111,179,173", paper: "194,114,47",
  };
  const THEME_INK: Record<ThemeId, string> = {
    beacon: "255,255,255", tide: "255,255,255", paper: "26,28,33",
  };
  let accentRgb = THEME_ACCENT.beacon;
  let inkRgb = THEME_INK.beacon;
  $effect(() => { accentRgb = THEME_ACCENT[settings.theme]; inkRgb = THEME_INK[settings.theme]; });

  // How hard the on-screen heading eases toward the sensor. Lower = steadier.
  const SMOOTH: Record<SmoothId, number> = {
    responsive: 0.22, balanced: 0.12, steady: 0.06,
  };

  $effect(() => {
    if (!ready) return;
    try { localStorage.setItem("signalume.settings", JSON.stringify(settings)); } catch {}
  });

  // Keep-screen-awake via the Wake Lock API (best-effort; ignored if absent).
  let wakeLock: any = null;
  $effect(() => {
    if (!ready) return;
    if (settings.keepAwake && !wakeLock) {
      (navigator as any).wakeLock?.request?.("screen")
        .then((w: any) => (wakeLock = w))
        .catch(() => {});
    } else if (!settings.keepAwake && wakeLock) {
      try { wakeLock.release(); } catch {}
      wakeLock = null;
    }
  });

  function haptic(pattern: number | number[]) {
    if (!settings.haptics) return;
    try { (navigator as any).vibrate?.(pattern); } catch {}
  }

  // ── Compass / heading ─────────────────────────────────────────────────
  let heading = $state(184);        // smoothed live heading used for math
  let shownHeading = $state(184);   // de-jittered value shown as text
  let headingTarget = 184;          // eased toward
  let sensorFiltered = 184;         // low-pass of the raw magnetometer
  let usingRealSensor = $state(false);

  function handleOrientation(
    e: DeviceOrientationEvent & { webkitCompassHeading?: number }
  ) {
    let compass: number | null = null;
    if (typeof e.webkitCompassHeading === "number") compass = e.webkitCompassHeading;
    else if (e.alpha != null) compass = (360 - e.alpha) % 360;
    if (compass == null) return;
    usingRealSensor = true;
    // Heavy low-pass on the *raw* magnetometer: it's noisy and jumps several
    // times a second. We ease toward it rather than snapping — this, plus the
    // display deadbands below, is what stops the numbers flickering.
    sensorFiltered = circLerp(sensorFiltered, compass, 0.18);
    headingTarget = sensorFiltered;
  }

  const COMPASS_POINTS = [
    "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE",
    "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW",
  ];
  let cardinal = $derived(COMPASS_POINTS[Math.round(shownHeading / 22.5) % 16]);

  // ── Simulated directional signal field ────────────────────────────────
  let bestBearing = 47;            // hidden "strongest direction" (drifts slowly)
  let qNoise = $state(0);          // small smoothed measurement noise

  // Raw quality 0..100: peaks when the phone faces bestBearing, dips facing away.
  let qualityRaw = $derived(
    clamp(Math.round(
      50 + 47 * Math.cos((angDiff(heading, bestBearing) * Math.PI) / 180) + qNoise
    ), 2, 99)
  );
  // Displayed strength with a deadband so it doesn't twitch ±1 forever.
  let quality = $state(50);
  $effect(() => { if (Math.abs(qualityRaw - quality) >= 2) quality = qualityRaw; });

  let qInfo = $derived.by(() => {
    const q = quality;
    if (q >= 85) return { label: "Excellent", cls: "good", level: 5 };
    if (q >= 68) return { label: "Strong",    cls: "good", level: 4 };
    if (q >= 45) return { label: "Fair",      cls: "fair", level: 3 };
    if (q >= 22) return { label: "Weak",      cls: "weak", level: 2 };
    return { label: "Very weak", cls: "weak", level: 1 };
  });

  // Where is the strongest direction relative to where I'm pointing?
  let toBest = $derived(angDiff(heading, bestBearing)); // + → turn right
  let guidance = $derived.by(() => {
    const d = Math.abs(toBest);
    if (d <= 12) return { arrow: "✓", main: "You're facing it", sub: "Strongest signal is right ahead", aligned: true };
    const right = toBest > 0;
    const deg = Math.round(d / 5) * 5; // calm, coarse turn amount
    return {
      arrow: right ? "→" : "←",
      main: right ? "Turn right" : "Turn left",
      sub: `about ${deg}° more`,
      aligned: false,
    };
  });

  // Buzz once at the moment you swing onto the strongest direction.
  let wasAligned = false;
  $effect(() => {
    const a = guidance.aligned;
    if (a && !wasAligned && settings.buzzOnLock) {
      try { (navigator as any).vibrate?.(35); } catch {}
    }
    wasAligned = a;
  });

  // Friendly technical readouts, all derived from the one quality value so they
  // always agree with the meter. Clearly simulated — shown only on request.
  let wifiDbm = $derived(Math.round(-92 + quality * 0.5));   // -92 .. -43
  let cellDbm = $derived(Math.round(-100 + quality * 0.4));  // -100 .. -60
  let cellPct = $derived(clamp(quality - 14, 0, 100));
  let pingMs = $derived(Math.round(58 - quality * 0.46));    // ~13 .. 56
  let speedMb = $derived(Math.round(8 + quality * 0.85));    // ~10 .. 92

  // ── Saved spots ("readings") ──────────────────────────────────────────
  type Pin = { id: number; bearing: number; quality: number; point: string };
  let pins = $state<Pin[]>([]);
  let pinId = 0;

  function takeReading() {
    haptic([12, 28, 12]);
    const b = Math.round(heading);
    const point = COMPASS_POINTS[Math.round(b / 22.5) % 16];
    pins = [...pins.slice(-15), { id: pinId++, bearing: b, quality, point }];
    showToast(`Spot saved · ${quality}% facing ${point}`);
  }
  let bestPin = $derived.by(() =>
    pins.reduce<Pin | null>((b, p) => (!b || p.quality > b.quality ? p : b), null)
  );
  let sortedPins = $derived([...pins].sort((a, b) => b.quality - a.quality));
  function clearPins() { pins = []; showToast("Saved spots cleared"); }

  function qualityClass(q: number) {
    return q >= 68 ? "good" : q >= 45 ? "fair" : "weak";
  }

  // ── Lightweight toast (replaces the old fake activity log) ─────────────
  let toast = $state<string | null>(null);
  let toastTimer: any;
  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 1900);
  }

  // ── Dial canvas ───────────────────────────────────────────────────────
  let canvas: HTMLCanvasElement;

  onMount(() => {
    // Load saved settings before enabling persistence.
    try {
      const raw = localStorage.getItem("signalume.settings");
      if (raw) settings = { ...settings, ...JSON.parse(raw) };
    } catch {}
    if (!THEME_META[settings.theme]) settings.theme = "beacon"; // migrate old ids
    ready = true;

    const ctx = canvas.getContext("2d")!;
    const DPR = window.devicePixelRatio || 1;
    const SIZE = 236;
    canvas.width = SIZE * DPR;
    canvas.height = SIZE * DPR;
    ctx.scale(DPR, DPR);
    const c = SIZE / 2;
    const R = c - 8;

    // Convert a bearing-relative-to-heading into a canvas point (up = aligned).
    const posFor = (bearingRel: number, r: number): [number, number] => {
      const a = ((bearingRel - 90) * Math.PI) / 180;
      return [c + Math.cos(a) * r, c + Math.sin(a) * r];
    };
    const sem = (q: number) =>
      q >= 68 ? "127,184,138" : q >= 45 ? "217,178,94" : "217,138,110";

    let raf = 0;
    function draw() {
      const ac = accentRgb;
      const ink = inkRgb;
      ctx.clearRect(0, 0, SIZE, SIZE);

      // Concentric range rings.
      ctx.strokeStyle = `rgba(${ink},0.07)`;
      ctx.lineWidth = 1;
      for (let i = 1; i <= 3; i++) {
        ctx.beginPath();
        ctx.arc(c, c, (R / 3) * i, 0, Math.PI * 2);
        ctx.stroke();
      }
      // Faint cross-hairs.
      ctx.strokeStyle = `rgba(${ink},0.05)`;
      ctx.beginPath();
      ctx.moveTo(c, c - R); ctx.lineTo(c, c + R);
      ctx.moveTo(c - R, c); ctx.lineTo(c + R, c);
      ctx.stroke();

      // A soft "up = aligned" target wedge at the top, so the goal is obvious.
      ctx.fillStyle = `rgba(${ac},0.06)`;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.arc(c, c, R - 2, (-90 - 14) * Math.PI / 180, (-90 + 14) * Math.PI / 180);
      ctx.closePath();
      ctx.fill();

      // North marker (rotates with the compass).
      const [nx, ny] = posFor(0 - heading, R - 2);
      ctx.fillStyle = `rgba(${ink},0.42)`;
      ctx.font = "600 11px ui-sans-serif, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      ctx.fillText("N", nx, ny);

      // Saved-spot markers around the dial.
      for (const p of pins) {
        const [x, y] = posFor(p.bearing - heading, R - 16);
        const rgb = sem(p.quality);
        const isBest = bestPin && p.id === bestPin.id;
        ctx.fillStyle = `rgba(${rgb},${isBest ? 0.95 : 0.5})`;
        ctx.beginPath();
        ctx.arc(x, y, isBest ? 4 : 2.6, 0, Math.PI * 2);
        ctx.fill();
        if (isBest) {
          ctx.strokeStyle = `rgba(${rgb},0.5)`;
          ctx.lineWidth = 1.5;
          ctx.beginPath();
          ctx.arc(x, y, 7, 0, Math.PI * 2);
          ctx.stroke();
        }
      }

      // The one arrow that matters: points to the strongest direction.
      const [tx, ty] = posFor(bestBearing - heading, R - 18);
      const lg = ctx.createLinearGradient(c, c, tx, ty);
      lg.addColorStop(0, `rgba(${ac},0.0)`);
      lg.addColorStop(1, `rgba(${ac},0.9)`);
      ctx.strokeStyle = lg;
      ctx.lineWidth = 3;
      ctx.lineCap = "round";
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.lineTo(tx, ty);
      ctx.stroke();
      // Arrow head.
      ctx.fillStyle = `rgba(${ac},1)`;
      ctx.beginPath();
      ctx.arc(tx, ty, 4.5, 0, Math.PI * 2);
      ctx.fill();

      // Center hub.
      ctx.fillStyle = `rgba(${ink},0.12)`;
      ctx.beginPath();
      ctx.arc(c, c, 3, 0, Math.PI * 2);
      ctx.fill();

      raf = requestAnimationFrame(draw);
    }
    raf = requestAnimationFrame(draw);

    // Sensors.
    window.addEventListener("deviceorientationabsolute", handleOrientation as EventListener);
    window.addEventListener("deviceorientation", handleOrientation as EventListener);
    const DOE = (window as any).DeviceOrientationEvent;
    if (DOE && typeof DOE.requestPermission === "function") DOE.requestPermission().catch(() => {});

    // Simulated heading wander when there's no real compass (desktop preview).
    const wanderTimer = setInterval(() => {
      if (usingRealSensor) return;
      headingTarget = (headingTarget + (Math.random() - 0.5) * 14 + 360) % 360;
    }, 2600);

    // Smoothly ease the on-screen heading; only restep the shown text past a
    // 2° deadband so the degree readout sits still instead of trembling.
    const easeTimer = setInterval(() => {
      heading = circLerp(heading, headingTarget, SMOOTH[settings.smoothing]);
      if (Math.abs(angDiff(shownHeading, heading)) >= 2) shownHeading = heading;
    }, 60);

    // The strongest direction drifts slowly so the app stays alive.
    const driftTimer = setInterval(() => {
      bestBearing = (bestBearing + (Math.random() - 0.5) * 6 + 360) % 360;
    }, 7000);

    // Gentle, smaller measurement noise on the quality reading.
    const noiseTimer = setInterval(() => {
      qNoise = qNoise * 0.7 + (Math.random() - 0.5) * 2.4;
    }, 900);

    return () => {
      window.removeEventListener("deviceorientationabsolute", handleOrientation as EventListener);
      window.removeEventListener("deviceorientation", handleOrientation as EventListener);
      cancelAnimationFrame(raf);
      clearInterval(wanderTimer);
      clearInterval(easeTimer);
      clearInterval(driftTimer);
      clearInterval(noiseTimer);
      clearTimeout(toastTimer);
    };
  });
</script>

<main class="app" data-theme={settings.theme}>
  <!-- ── HEADER ──────────────────────────────────────────────────────── -->
  <header class="topbar">
    <span class="brand"><span class="mark">◎</span> Signalume</span>
    <div class="top-actions">
      <span class="status" class:live={usingRealSensor}>
        <span class="dot"></span>{usingRealSensor ? "Live" : "Demo"}
      </span>
      <button class="icon-btn" aria-label="Settings" onclick={() => (settingsOpen = true)}>⚙</button>
    </div>
  </header>

  <!-- ── FINDER ──────────────────────────────────────────────────────── -->
  <section class="card finder">
    <div class="dial-wrap">
      <canvas bind:this={canvas} class="dial" style="width:236px;height:236px;"></canvas>
      <div class="dial-center">
        <div class="q-cap">SIGNAL</div>
        <div class="q-label {qInfo.cls}">{qInfo.label}</div>
        <div class="segs" aria-hidden="true">
          {#each [1, 2, 3, 4, 5] as i}
            <span class="seg {qInfo.cls}" class:on={i <= qInfo.level}></span>
          {/each}
        </div>
        <div class="q-head">{quality}% · {Math.round(shownHeading)}° {cardinal}</div>
      </div>
    </div>

    <!-- The single, plain-language instruction. -->
    <div class="guide" class:aligned={guidance.aligned}>
      <span class="guide-arrow">{guidance.arrow}</span>
      <span class="guide-text">
        <b>{guidance.main}</b>
        <small>{guidance.sub}</small>
      </span>
    </div>

    <button class="primary-btn" onclick={takeReading}>Save this spot</button>
    <p class="hint">
      Turn slowly until the arrow points up and it says you're facing it. Then
      tap <b>Save this spot</b> to compare places and find the best one.
    </p>
  </section>

  <!-- ── SAVED SPOTS ─────────────────────────────────────────────────── -->
  <section class="card spots">
    <div class="card-head">
      <span>Saved spots</span>
      {#if pins.length}<button class="link" onclick={clearPins}>Clear</button>{/if}
    </div>
    {#if !pins.length}
      <p class="empty">No spots yet. Walk around and tap <b>Save this spot</b> in a few places — the strongest one gets a ★.</p>
    {:else}
      <div class="spot-list">
        {#each sortedPins as p (p.id)}
          <div class="spot" class:best={bestPin && p.id === bestPin.id}>
            <span class="spot-dir">{p.point} · {p.bearing}°</span>
            <div class="bar sm"><div class="fill {qualityClass(p.quality)}" style="width:{p.quality}%"></div></div>
            <span class="spot-q {qualityClass(p.quality)}">{p.quality}%</span>
            {#if bestPin && p.id === bestPin.id}<span class="star">★</span>{/if}
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- ── TECHNICAL DETAILS (optional) ────────────────────────────────── -->
  {#if settings.showDetails}
    <section class="card details">
      <div class="card-head plain"><span>Details</span><span class="sim-tag">simulated</span></div>
      <div class="det-body">
        <div class="row">
          <span class="r-label">Wi-Fi</span>
          <div class="bar"><div class="fill" style="width:{quality}%"></div></div>
          <span class="r-val">{wifiDbm}<small>dBm</small></span>
        </div>
        <div class="row">
          <span class="r-label">Cellular</span>
          <div class="bar"><div class="fill" style="width:{cellPct}%"></div></div>
          <span class="r-val">{cellDbm}<small>dBm</small></span>
        </div>
        <div class="mini-row">
          <div class="mini"><span class="mini-l">Ping</span><span class="mini-v">{pingMs}<small>ms</small></span></div>
          <div class="mini"><span class="mini-l">Speed</span><span class="mini-v">{speedMb}<small>Mb/s</small></span></div>
          <div class="mini"><span class="mini-l">Heading</span><span class="mini-v">{Math.round(shownHeading)}<small>°</small></span></div>
        </div>
      </div>
    </section>
  {/if}
</main>

<!-- ── TOAST ─────────────────────────────────────────────────────────── -->
{#if toast}
  <div class="toast" data-theme={settings.theme} role="status">{toast}</div>
{/if}

<!-- ── SETTINGS SHEET ────────────────────────────────────────────────── -->
{#if settingsOpen}
  <div class="sheet-backdrop" data-theme={settings.theme} onclick={() => (settingsOpen = false)} role="presentation">
    <div class="sheet" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Settings">
      <div class="sheet-head">
        <span>Settings</span>
        <button class="icon-btn" aria-label="Close" onclick={() => (settingsOpen = false)}>✕</button>
      </div>

      <div class="set-group">
        <div class="set-title">Theme</div>
        <div class="themes">
          {#each Object.entries(THEME_META) as [id, meta]}
            <button class="theme-chip" class:on={settings.theme === id}
                    onclick={() => (settings.theme = id as ThemeId)}>
              <span class="sw" style="background:{meta.swatch}"></span>{meta.label}
            </button>
          {/each}
        </div>
      </div>

      <div class="set-group">
        <div class="set-title">Steadiness</div>
        <div class="seg-pick">
          {#each [["responsive","Responsive"],["balanced","Balanced"],["steady","Steady"]] as [id, label]}
            <button class="seg-opt" class:on={settings.smoothing === id}
                    onclick={() => (settings.smoothing = id as SmoothId)}>{label}</button>
          {/each}
        </div>
        <p class="set-note">How much the compass is smoothed. Pick <b>Steady</b> if the numbers jump around too much.</p>
      </div>

      <div class="set-group">
        <button class="toggle" onclick={() => (settings.buzzOnLock = !settings.buzzOnLock)}>
          <span><b>Buzz when aligned</b><small>Short vibration the instant you face the strongest signal</small></span>
          <span class="sw-track" class:on={settings.buzzOnLock}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.haptics = !settings.haptics)}>
          <span><b>Haptic feedback</b><small>Vibrate when saving a spot</small></span>
          <span class="sw-track" class:on={settings.haptics}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.showDetails = !settings.showDetails)}>
          <span><b>Show technical details</b><small>Wi-Fi / cellular / ping numbers (simulated)</small></span>
          <span class="sw-track" class:on={settings.showDetails}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.keepAwake = !settings.keepAwake)}>
          <span><b>Keep screen on</b><small>Stop the display sleeping while hunting</small></span>
          <span class="sw-track" class:on={settings.keepAwake}><span class="knob"></span></span>
        </button>
      </div>

      <div class="set-group">
        <button class="danger" onclick={() => clearPins()}>Clear saved spots</button>
      </div>

      <div class="set-group howto">
        <div class="set-title">How it works</div>
        <p>Hold the phone flat and turn slowly. The big word is how strong the
          signal is where you're pointing; the bars fill as it gets stronger.
          The arrow on the dial always points to the strongest direction — turn
          until it's at the top and it reads <b>You're facing it</b>.</p>
        <p><b>Save this spot</b> pins where you're standing. The list shows every
          spot, strongest first, with a ★ on the best — handy for picking where
          to sit or place a router.</p>
        <p class="howto-note">This is a prototype: the signal reading is
          simulated, so it's for showing the interaction, not measuring your real
          network yet.</p>
      </div>

      <div class="ver">Signalume · prototype</div>
    </div>
  </div>
{/if}

<style>
  :global(html, body) {
    margin: 0;
    background: #14161b;
  }
  /* Natural document scroll — the most reliable way to reach lower cards on a
     phone. (The old fixed-height + inner overflow trick failed in the webview.) */
  :global(body) { min-height: 100dvh; }

  .app {
    /* ── Beacon (default): warm lighthouse amber on deep ink ── */
    --bg: #14161b;
    --surface: #1b1e25;
    --surface-2: #232730;
    --line: rgba(255, 255, 255, 0.06);
    --text: #e7e9ee;
    --muted: #8b919e;
    --accent: #e0a04a;
    --good: #7fb88a;
    --fair: #d9b25e;
    --weak: #d98a6e;

    min-height: 100dvh;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 12px;
    /* Respect the status bar / nav bar so content clears system icons. */
    padding:
      calc(16px + env(safe-area-inset-top)) calc(16px + env(safe-area-inset-right))
      calc(24px + env(safe-area-inset-bottom)) calc(16px + env(safe-area-inset-left));
    background: var(--bg);
    color: var(--text);
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
  }

  /* ── Tide: cool, calm desaturated teal ── */
  .app[data-theme="tide"] {
    --bg: #101417; --surface: #171c20; --surface-2: #1f262b;
    --line: rgba(255, 255, 255, 0.06);
    --text: #e6ecef; --muted: #859098; --accent: #6fb3ad;
    --good: #7fb8a0; --fair: #d9b25e; --weak: #d98a6e;
  }
  /* ── Paper: light, daylight-readable field instrument ── */
  .app[data-theme="paper"] {
    --bg: #ece8e1; --surface: #f5f2ea; --surface-2: #e3ddd1;
    --line: rgba(0, 0, 0, 0.09);
    --text: #2a2c30; --muted: #6b6f78; --accent: #c2722f;
    --good: #4f8f5e; --fair: #ab7f28; --weak: #b65a3e;
  }
  :global(body:has(.app[data-theme="paper"])) { background: #ece8e1; }

  .card {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 16px;
    flex-shrink: 0;
  }

  /* ── Header ── */
  .topbar { display: flex; justify-content: space-between; align-items: center; padding: 2px 2px; }
  .brand { font-size: 17px; font-weight: 600; letter-spacing: 0.2px; display: inline-flex; align-items: center; gap: 7px; }
  .mark { color: var(--accent); font-size: 18px; }
  .top-actions { display: flex; align-items: center; gap: 8px; }
  .status {
    display: inline-flex; align-items: center; gap: 6px;
    font-size: 12px; color: var(--muted);
    background: var(--surface); border: 1px solid var(--line);
    padding: 5px 11px; border-radius: 999px;
  }
  .status.live { color: var(--good); }
  .dot { width: 6px; height: 6px; border-radius: 50%; background: currentColor; animation: pulse 1.8s ease-in-out infinite; }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.35; } }
  .icon-btn {
    width: 34px; height: 34px; border-radius: 10px;
    background: var(--surface); border: 1px solid var(--line);
    color: var(--text); font-size: 16px; cursor: pointer; line-height: 1;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .icon-btn:active { background: var(--surface-2); }

  /* ── Finder ── */
  .finder { display: flex; flex-direction: column; align-items: center; gap: 14px; padding: 22px 18px; }
  .dial-wrap { position: relative; width: 236px; height: 236px; }
  .dial { display: block; }
  .dial-center {
    position: absolute; inset: 0; pointer-events: none;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .q-cap { font-size: 10px; letter-spacing: 3px; color: var(--muted); }
  .q-label { font-size: 30px; font-weight: 700; line-height: 1; margin-top: 4px; letter-spacing: 0.2px; }
  .segs { display: flex; gap: 5px; margin-top: 12px; }
  .seg { width: 16px; height: 6px; border-radius: 3px; background: color-mix(in srgb, var(--text) 12%, transparent); transition: background 0.35s ease; }
  .seg.on.good { background: var(--good); }
  .seg.on.fair { background: var(--fair); }
  .seg.on.weak { background: var(--weak); }
  .q-head { font-size: 11px; color: var(--muted); margin-top: 12px; font-variant-numeric: tabular-nums; }
  .good { color: var(--good); }
  .fair { color: var(--fair); }
  .weak { color: var(--weak); }

  .guide {
    display: inline-flex; align-items: center; gap: 12px;
    width: 100%; box-sizing: border-box;
    background: var(--surface-2); border: 1px solid var(--line);
    padding: 12px 16px; border-radius: 14px;
  }
  .guide.aligned { background: color-mix(in srgb, var(--good) 12%, var(--surface-2)); border-color: color-mix(in srgb, var(--good) 35%, transparent); }
  .guide-arrow { font-size: 26px; line-height: 1; color: var(--accent); flex-shrink: 0; width: 28px; text-align: center; }
  .guide.aligned .guide-arrow { color: var(--good); }
  .guide-text { display: flex; flex-direction: column; gap: 2px; }
  .guide-text b { font-size: 16px; font-weight: 600; }
  .guide-text small { font-size: 12px; color: var(--muted); }
  .guide.aligned .guide-text b { color: var(--good); }

  .primary-btn {
    width: 100%; font: inherit; font-size: 15px; font-weight: 600;
    color: var(--bg); background: var(--accent);
    border: none; border-radius: 12px; padding: 14px; cursor: pointer;
    letter-spacing: 0.2px;
  }
  .primary-btn:active { filter: brightness(0.92); transform: translateY(1px); }
  .hint { margin: 0; font-size: 11.5px; color: var(--muted); text-align: center; line-height: 1.55; max-width: 290px; }
  .hint b { color: var(--text); }

  /* ── Card heads / spots ── */
  .card-head { display: flex; justify-content: space-between; align-items: center; padding: 13px 16px; font-size: 13px; font-weight: 600; }
  .card-head.plain { padding-bottom: 0; }
  .sim-tag { font-size: 10px; font-weight: 600; letter-spacing: 0.6px; text-transform: uppercase; color: var(--muted); background: color-mix(in srgb, var(--text) 7%, transparent); padding: 2px 7px; border-radius: 6px; }
  .link { font: inherit; font-size: 12px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 0; }
  .empty { margin: 0; padding: 0 16px 16px; font-size: 12px; color: var(--muted); line-height: 1.55; }
  .empty b { color: var(--text); }
  .spot-list { display: flex; flex-direction: column; padding-bottom: 6px; }
  .spot { display: flex; align-items: center; gap: 11px; padding: 9px 16px; }
  .spot + .spot { border-top: 1px solid var(--line); }
  .spot.best { background: color-mix(in srgb, var(--good) 8%, transparent); }
  .spot-dir { font-size: 12px; width: 78px; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .spot-q { font-size: 12px; width: 36px; text-align: right; font-variant-numeric: tabular-nums; font-weight: 600; }
  .star { color: var(--good); font-size: 12px; }

  /* ── Details ── */
  .det-body { padding: 14px 16px 16px; display: flex; flex-direction: column; gap: 14px; }
  .row { display: flex; align-items: center; gap: 12px; }
  .r-label { font-size: 12px; color: var(--muted); width: 56px; flex-shrink: 0; }
  .r-val { font-size: 12px; color: var(--text); width: 64px; text-align: right; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .r-val small, .mini-v small { font-size: 10px; color: var(--muted); margin-left: 2px; }
  .bar { flex: 1; height: 7px; border-radius: 999px; background: color-mix(in srgb, var(--text) 8%, transparent); overflow: hidden; }
  .bar.sm { height: 5px; }
  .fill { height: 100%; border-radius: 999px; background: var(--accent); transition: width 0.5s cubic-bezier(0.22,1,0.36,1); }
  .fill.good { background: var(--good); } .fill.fair { background: var(--fair); } .fill.weak { background: var(--weak); }
  .mini-row { display: flex; gap: 10px; }
  .mini { flex: 1; background: var(--surface-2); border-radius: 12px; padding: 11px 8px; display: flex; flex-direction: column; gap: 4px; align-items: center; }
  .mini-l { font-size: 10px; color: var(--muted); }
  .mini-v { font-size: 16px; font-weight: 700; font-variant-numeric: tabular-nums; }

  /* ── Toast ── */
  .toast {
    position: fixed; left: 50%; transform: translateX(-50%);
    bottom: calc(22px + env(safe-area-inset-bottom)); z-index: 60;
    background: var(--surface-2, #232730); color: var(--text, #e7e9ee);
    border: 1px solid var(--line, rgba(255,255,255,0.06));
    padding: 10px 16px; border-radius: 999px; font-size: 12.5px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.3);
    animation: toast-in 0.2s ease;
  }
  @keyframes toast-in { from { opacity: 0; transform: translate(-50%, 6px); } to { opacity: 1; transform: translate(-50%, 0); } }

  /* ── Settings sheet ── */
  .sheet-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: flex-end; justify-content: center; z-index: 50; }
  .sheet {
    width: 100%; max-width: 480px; max-height: 88dvh; overflow-y: auto;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--line); border-bottom: none;
    border-radius: 20px 20px 0 0;
    padding: 16px 18px calc(22px + env(safe-area-inset-bottom));
    display: flex; flex-direction: column; gap: 18px;
    font-family: ui-sans-serif, system-ui, sans-serif;
    animation: rise 0.22s cubic-bezier(0.22,1,0.36,1);
  }
  @keyframes rise { from { transform: translateY(20px); opacity: 0.6; } to { transform: translateY(0); opacity: 1; } }
  .sheet-head { display: flex; justify-content: space-between; align-items: center; font-size: 16px; font-weight: 600; }
  .set-group { display: flex; flex-direction: column; gap: 10px; }
  .set-title { font-size: 11px; letter-spacing: 1.5px; text-transform: uppercase; color: var(--muted); }
  .set-note { margin: 0; font-size: 11.5px; color: var(--muted); line-height: 1.5; }
  .set-note b { color: var(--text); }
  .themes { display: flex; gap: 8px; }
  .theme-chip {
    flex: 1; display: inline-flex; align-items: center; justify-content: center; gap: 7px;
    font: inherit; font-size: 13px; color: var(--text);
    background: var(--surface); border: 1px solid var(--line);
    padding: 10px; border-radius: 11px; cursor: pointer;
  }
  .theme-chip.on { border-color: var(--accent); background: var(--surface-2); }
  .sw { width: 12px; height: 12px; border-radius: 4px; }
  .seg-pick { display: flex; gap: 6px; background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 4px; }
  .seg-opt { flex: 1; font: inherit; font-size: 13px; color: var(--muted); background: none; border: none; padding: 9px 4px; border-radius: 9px; cursor: pointer; }
  .seg-opt.on { color: var(--text); background: var(--surface-2); }
  .toggle {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    font: inherit; text-align: left; color: var(--text);
    background: var(--surface); border: 1px solid var(--line);
    padding: 12px 14px; border-radius: 12px; cursor: pointer;
  }
  .toggle b { display: block; font-size: 14px; font-weight: 600; }
  .toggle small { display: block; font-size: 11px; color: var(--muted); margin-top: 2px; }
  .sw-track { width: 42px; height: 24px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); position: relative; flex-shrink: 0; transition: background 0.2s; }
  .sw-track.on { background: var(--accent); }
  .knob { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: var(--bg); transition: transform 0.2s; }
  .sw-track.on .knob { transform: translateX(18px); background: var(--bg); }
  .danger {
    font: inherit; font-size: 14px; color: var(--weak);
    background: var(--surface); border: 1px solid color-mix(in srgb, var(--weak) 30%, var(--line));
    padding: 12px; border-radius: 12px; cursor: pointer;
  }
  .howto p { margin: 0; font-size: 12.5px; color: var(--muted); line-height: 1.6; }
  .howto p + p { margin-top: 8px; }
  .howto b { color: var(--text); }
  .howto-note { font-style: italic; opacity: 0.85; }
  .ver { text-align: center; font-size: 11px; color: var(--muted); opacity: 0.7; }
</style>
