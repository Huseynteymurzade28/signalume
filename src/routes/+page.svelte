<script lang="ts">
  import { onMount } from "svelte";

  // ── Compass / heading (real device sensor, simulated fallback) ────────
  let heading = $state(184);          // degrees 0..359 — smoothed value on screen
  let headingTarget = 184;            // value the screen eases toward
  let usingRealSensor = $state(false); // true once real orientation data arrives

  // The Web DeviceOrientation API exposes the phone's magnetometer + gyroscope
  // fusion. `alpha` (or webkitCompassHeading on iOS) gives the compass heading.
  function handleOrientation(
    e: DeviceOrientationEvent & { webkitCompassHeading?: number }
  ) {
    let compass: number | null = null;
    if (typeof e.webkitCompassHeading === "number") {
      compass = e.webkitCompassHeading;   // iOS: already a compass heading
    } else if (e.alpha != null) {
      compass = (360 - e.alpha) % 360;     // Android: derive from alpha
    }
    if (compass == null) return;
    usingRealSensor = true;
    headingTarget = (compass + 360) % 360;
  }

  const COMPASS_POINTS = [
    "N", "NNE", "NE", "ENE", "E", "ESE", "SE", "SSE",
    "S", "SSW", "SW", "WSW", "W", "WNW", "NW", "NNW",
  ];
  const CARDINAL_NAMES: Record<string, string> = {
    N: "North", NE: "North-East", E: "East", SE: "South-East",
    S: "South", SW: "South-West", W: "West", NW: "North-West",
  };

  let cardinal = $derived(COMPASS_POINTS[Math.round(heading / 22.5) % 16]);
  let cardinalLong = $derived(
    CARDINAL_NAMES[cardinal] ??
      (CARDINAL_NAMES[cardinal.slice(0, 2) as string] ?? cardinal)
  );

  // ── Live signal metrics (simulated) ───────────────────────────────────
  let wifiDbm = $state(-57);   // -50 .. -65 dBm
  let cellDbm = $state(-82);   // -70 .. -95 dBm
  let pingMs = $state(16);     // 10 .. 25 ms
  let throughput = $state(48); // Mbps

  // Map dBm to a 0..100 strength percentage for the bars.
  function dbmToPct(dbm: number, min: number, max: number) {
    return Math.max(0, Math.min(100, ((dbm - min) / (max - min)) * 100));
  }
  let wifiPct = $derived(dbmToPct(wifiDbm, -90, -40));
  let cellPct = $derived(dbmToPct(cellDbm, -100, -60));

  // ── Activity log stream (simulated) ───────────────────────────────────
  type LogLine = { id: number; ts: string; level: string; msg: string };
  let logs = $state<LogLine[]>([]);
  let logId = 0;
  let logBox: HTMLDivElement;

  const LOG_TEMPLATES: { level: string; msg: string }[] = [
    { level: "INFO", msg: "Scanning frequencies 2.4GHz / 5GHz" },
    { level: "OK", msg: "Connected to gateway 192.168.0.1" },
    { level: "INFO", msg: "Triangulating signal source" },
    { level: "WARN", msg: "Jitter detected on channel 11" },
    { level: "OK", msg: "Handshake complete — link stable" },
    { level: "INFO", msg: "Sweeping sector for beacons" },
    { level: "WARN", msg: "Packet loss spike: 2.1%" },
    { level: "INFO", msg: "Calibrating magnetometer" },
    { level: "OK", msg: "GPS lock acquired (8 satellites)" },
    { level: "ERR", msg: "Beacon timeout — retrying" },
    { level: "INFO", msg: "Decoding SSID broadcast" },
    { level: "OK", msg: "Encryption negotiated: WPA3" },
  ];

  function pushLog() {
    const t = LOG_TEMPLATES[Math.floor(Math.random() * LOG_TEMPLATES.length)];
    const now = new Date();
    const ts =
      String(now.getHours()).padStart(2, "0") + ":" +
      String(now.getMinutes()).padStart(2, "0") + ":" +
      String(now.getSeconds()).padStart(2, "0");
    logs = [...logs.slice(-40), { id: logId++, ts, level: t.level, msg: t.msg }];
  }

  // Auto-scroll the log to the bottom whenever it changes.
  $effect(() => {
    logs; // track
    if (logBox) logBox.scrollTop = logBox.scrollHeight;
  });

  // ── Radar canvas ──────────────────────────────────────────────────────
  let canvas: HTMLCanvasElement;

  type Blip = { angle: number; radius: number; born: number; life: number };

  onMount(() => {
    const ctx = canvas.getContext("2d")!;
    const DPR = window.devicePixelRatio || 1;
    const SIZE = 240;
    canvas.width = SIZE * DPR;
    canvas.height = SIZE * DPR;
    ctx.scale(DPR, DPR);
    const c = SIZE / 2;
    const R = c - 6;

    let sweep = 0; // radians
    let blips: Blip[] = [];
    let raf = 0;

    function spawnBlip() {
      blips.push({
        angle: Math.random() * Math.PI * 2,
        radius: 20 + Math.random() * (R - 30),
        born: performance.now(),
        life: 2400 + Math.random() * 1800,
      });
    }

    function draw(now: number) {
      ctx.clearRect(0, 0, SIZE, SIZE);

      // Soft grid rings
      ctx.strokeStyle = "rgba(192,202,245,0.10)";
      ctx.lineWidth = 1;
      for (let i = 1; i <= 4; i++) {
        ctx.beginPath();
        ctx.arc(c, c, (R / 4) * i, 0, Math.PI * 2);
        ctx.stroke();
      }
      // Soft cross-hairs
      ctx.beginPath();
      ctx.moveTo(c, c - R); ctx.lineTo(c, c + R);
      ctx.moveTo(c - R, c); ctx.lineTo(c + R, c);
      ctx.stroke();

      // Gentle sweep wedge (soft cyan)
      const grad = ctx.createConicGradient(sweep - Math.PI / 2, c, c);
      grad.addColorStop(0, "rgba(125,207,255,0.0)");
      grad.addColorStop(0.9, "rgba(125,207,255,0.0)");
      grad.addColorStop(1, "rgba(125,207,255,0.20)");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.arc(c, c, R, 0, Math.PI * 2);
      ctx.fill();

      // Sweep line
      ctx.strokeStyle = "rgba(125,207,255,0.65)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.lineTo(c + Math.cos(sweep - Math.PI / 2) * R, c + Math.sin(sweep - Math.PI / 2) * R);
      ctx.stroke();

      // Blips (soft green, fade over their lifetime)
      blips = blips.filter((b) => now - b.born < b.life);
      for (const b of blips) {
        const age = (now - b.born) / b.life;
        const alpha = Math.sin((1 - age) * Math.PI) * 0.85 + 0.1;
        const x = c + Math.cos(b.angle - Math.PI / 2) * b.radius;
        const y = c + Math.sin(b.angle - Math.PI / 2) * b.radius;
        ctx.fillStyle = `rgba(158,206,106,${alpha})`;
        ctx.shadowColor = "rgba(158,206,106,0.7)";
        ctx.shadowBlur = 6;
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;
      }

      sweep = (sweep + 0.018) % (Math.PI * 2);
      raf = requestAnimationFrame(draw);
    }
    raf = requestAnimationFrame(draw);

    // ── Simulation timers ──────────────────────────────────────────────
    const blipTimer = setInterval(() => {
      if (Math.random() > 0.35) spawnBlip();
    }, 700);

    // Try to read the real device orientation. On Android the "absolute" event
    // is magnetometer-referenced (true compass); fall back to the relative one.
    window.addEventListener("deviceorientationabsolute", handleOrientation as EventListener);
    window.addEventListener("deviceorientation", handleOrientation as EventListener);
    // iOS 13+ needs an explicit permission request (no-op on Android).
    const DOE = (window as any).DeviceOrientationEvent;
    if (DOE && typeof DOE.requestPermission === "function") {
      DOE.requestPermission().catch(() => {});
    }

    // Fallback: only wander the heading while NO real sensor data arrives
    // (e.g. a desktop browser with no compass).
    const headingTimer = setInterval(() => {
      if (usingRealSensor) return;
      headingTarget = (headingTarget + (Math.random() - 0.5) * 24 + 360) % 360;
    }, 1500);

    const easeTimer = setInterval(() => {
      let diff = ((headingTarget - heading + 540) % 360) - 180;
      heading = (heading + diff * 0.08 + 360) % 360;
    }, 50);

    const metricTimer = setInterval(() => {
      const jit = (n: number, a: number, b: number, step: number) =>
        Math.max(a, Math.min(b, n + (Math.random() - 0.5) * step));
      wifiDbm = Math.round(jit(wifiDbm, -65, -50, 6));
      cellDbm = Math.round(jit(cellDbm, -95, -70, 8));
      pingMs = Math.round(jit(pingMs, 10, 25, 6));
      throughput = Math.round(jit(throughput, 22, 72, 14));
    }, 800);

    const logTimer = setInterval(pushLog, 1800);
    pushLog();

    return () => {
      window.removeEventListener("deviceorientationabsolute", handleOrientation as EventListener);
      window.removeEventListener("deviceorientation", handleOrientation as EventListener);
      cancelAnimationFrame(raf);
      clearInterval(blipTimer);
      clearInterval(headingTimer);
      clearInterval(easeTimer);
      clearInterval(metricTimer);
      clearInterval(logTimer);
    };
  });
</script>

<main class="app">
  <!-- ── HEADER ─────────────────────────────────────────────────────── -->
  <header class="topbar">
    <span class="brand">🧭 Signalume</span>
    <span class="status" class:live={usingRealSensor}>
      <span class="dot"></span>
      {usingRealSensor ? "Live sensor" : "Simulated"}
    </span>
  </header>

  <!-- ── TOP: RADAR / COMPASS ───────────────────────────────────────── -->
  <section class="card radar-card">
    <div class="radar-box">
      <canvas bind:this={canvas} class="radar" style="width:240px;height:240px;"></canvas>
      <div class="radar-center">
        <div class="heading-label">Heading</div>
        <div class="heading">{Math.round(heading)}°</div>
        <div class="cardinal">{cardinalLong}</div>
      </div>
      <span class="tick n">N</span>
      <span class="tick e">E</span>
      <span class="tick s">S</span>
      <span class="tick w">W</span>
    </div>
  </section>

  <!-- ── MIDDLE: SIGNAL METRICS ─────────────────────────────────────── -->
  <section class="card metrics">
    <div class="metric">
      <div class="metric-head">
        <span class="m-label">Wi-Fi signal</span>
        <span class="m-val">{wifiDbm} <small>dBm</small></span>
      </div>
      <div class="bar"><div class="fill cyan" style="width:{wifiPct}%"></div></div>
    </div>

    <div class="metric">
      <div class="metric-head">
        <span class="m-label">Cellular</span>
        <span class="m-val">{cellDbm} <small>dBm</small></span>
      </div>
      <div class="bar"><div class="fill purple" style="width:{cellPct}%"></div></div>
    </div>

    <div class="mini-row">
      <div class="mini">
        <span class="mini-label">Ping</span>
        <span class="mini-val">{pingMs}<small>ms</small></span>
      </div>
      <div class="mini">
        <span class="mini-label">Speed</span>
        <span class="mini-val">{throughput}<small>Mb/s</small></span>
      </div>
      <div class="mini">
        <span class="mini-label">Status</span>
        <span class="mini-val ok">Online</span>
      </div>
    </div>
  </section>

  <!-- ── BOTTOM: ACTIVITY LOG ───────────────────────────────────────── -->
  <section class="card log-card">
    <div class="log-head">
      <span>Activity log</span>
      <span class="live-pill"><span class="dot"></span> Live</span>
    </div>
    <div class="log-body" bind:this={logBox}>
      {#each logs as line (line.id)}
        <div class="log">
          <span class="ts">{line.ts}</span>
          <span class="chip {line.level.toLowerCase()}">{line.level}</span>
          <span class="txt">{line.msg}</span>
        </div>
      {/each}
    </div>
  </section>
</main>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden; /* no global scrolling */
    background: #1a1b26;
  }

  .app {
    --bg: #1a1b26;
    --card: #232739;
    --card-soft: #2a2f44;
    --line: rgba(192, 202, 245, 0.08);
    --text: #c0caf5;
    --muted: #8a90b8;
    --cyan: #7dcfff;
    --purple: #bb9af3;
    --green: #9ece6a;
    --red: #f7768e;
    --yellow: #e0af68;

    height: 100vh;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 18px 16px;
    background:
      radial-gradient(900px 500px at 50% -10%, #2a2f47 0%, rgba(42, 47, 71, 0) 60%),
      var(--bg);
    color: var(--text);
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
  }

  .card {
    background: var(--card);
    border: 1px solid var(--line);
    border-radius: 20px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.25);
  }

  /* ── Header ── */
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 4px;
  }
  .brand { font-size: 18px; font-weight: 600; letter-spacing: 0.2px; }
  .status {
    display: flex; align-items: center; gap: 7px;
    font-size: 12px; color: var(--muted);
    background: var(--card-soft);
    padding: 6px 12px; border-radius: 999px;
  }
  .status.live { color: var(--green); }
  .dot {
    width: 7px; height: 7px; border-radius: 50%;
    background: currentColor; opacity: 0.9;
    animation: pulse 1.6s ease-in-out infinite;
  }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.3; } }

  /* ── Radar ── */
  .radar-card { display: flex; justify-content: center; padding: 18px; }
  .radar-box {
    position: relative;
    width: 240px; height: 240px;
    border-radius: 50%;
    background:
      radial-gradient(circle at 50% 50%, rgba(125,207,255,0.05), rgba(0,0,0,0.25));
    border: 1px solid rgba(125, 207, 255, 0.14);
  }
  .radar { display: block; }
  .radar-center {
    position: absolute; inset: 0;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    pointer-events: none;
  }
  .heading-label {
    font-size: 11px; color: var(--muted); margin-bottom: 2px;
  }
  .heading {
    font-size: 40px; font-weight: 700; color: #d6ecff;
    font-variant-numeric: tabular-nums;
    text-shadow: 0 0 16px rgba(125, 207, 255, 0.35);
    line-height: 1;
  }
  .cardinal { font-size: 13px; color: var(--cyan); margin-top: 4px; font-weight: 500; }
  .tick {
    position: absolute; font-size: 11px; color: var(--muted); font-weight: 600;
  }
  .tick.n { top: 10px; left: 50%; transform: translateX(-50%); }
  .tick.s { bottom: 10px; left: 50%; transform: translateX(-50%); }
  .tick.e { right: 12px; top: 50%; transform: translateY(-50%); }
  .tick.w { left: 12px; top: 50%; transform: translateY(-50%); }

  /* ── Metrics ── */
  .metrics {
    padding: 18px;
    display: flex; flex-direction: column; gap: 16px;
  }
  .metric-head {
    display: flex; justify-content: space-between; align-items: baseline;
    margin-bottom: 8px;
  }
  .m-label { font-size: 13px; color: var(--muted); }
  .m-val {
    font-size: 15px; font-weight: 600;
    font-variant-numeric: tabular-nums;
  }
  .m-val small, .mini-val small { font-size: 11px; color: var(--muted); font-weight: 400; margin-left: 2px; }
  .bar {
    height: 10px; border-radius: 999px;
    background: rgba(192, 202, 245, 0.08); overflow: hidden;
  }
  .fill {
    height: 100%; border-radius: 999px;
    transition: width 0.6s cubic-bezier(0.22, 1, 0.36, 1);
  }
  .fill.cyan { background: linear-gradient(90deg, #5aa9e6, var(--cyan)); }
  .fill.purple { background: linear-gradient(90deg, #9d7cd8, var(--purple)); }

  .mini-row { display: flex; gap: 10px; }
  .mini {
    flex: 1;
    background: var(--card-soft);
    border-radius: 14px;
    padding: 12px 10px;
    display: flex; flex-direction: column; gap: 6px;
    align-items: center;
  }
  .mini-label { font-size: 11px; color: var(--muted); }
  .mini-val {
    font-size: 17px; font-weight: 700;
    font-variant-numeric: tabular-nums;
  }
  .mini-val.ok { color: var(--green); }

  /* ── Activity log ── */
  .log-card {
    flex: 1; min-height: 0;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .log-head {
    display: flex; justify-content: space-between; align-items: center;
    padding: 14px 18px;
    font-size: 13px; font-weight: 600; color: var(--text);
    border-bottom: 1px solid var(--line);
  }
  .live-pill {
    display: flex; align-items: center; gap: 6px;
    font-size: 11px; font-weight: 500; color: var(--green);
  }
  .log-body {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 10px 14px;
    font-family: ui-monospace, "JetBrains Mono", "SFMono-Regular", "Fira Code", monospace;
    font-size: 12px;
    line-height: 2;
    scrollbar-width: thin;
    scrollbar-color: #353b54 transparent;
  }
  .log-body::-webkit-scrollbar { width: 6px; }
  .log-body::-webkit-scrollbar-thumb { background: #353b54; border-radius: 999px; }

  .log { display: flex; align-items: center; gap: 8px; white-space: nowrap; }
  .ts { color: var(--muted); font-size: 11px; }
  .chip {
    font-size: 10px; font-weight: 700; letter-spacing: 0.3px;
    padding: 1px 7px; border-radius: 999px;
    flex-shrink: 0;
  }
  .chip.info { color: var(--cyan); background: rgba(125, 207, 255, 0.12); }
  .chip.ok { color: var(--green); background: rgba(158, 206, 106, 0.12); }
  .chip.warn { color: var(--yellow); background: rgba(224, 175, 104, 0.12); }
  .chip.err { color: var(--red); background: rgba(247, 118, 142, 0.12); }
  .txt { color: #a9b1d6; }
</style>
