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
    N: "NORTH", NE: "NORTH-EAST", E: "EAST", SE: "SOUTH-EAST",
    S: "SOUTH", SW: "SOUTH-WEST", W: "WEST", NW: "NORTH-WEST",
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

  // ── Terminal log stream (simulated) ───────────────────────────────────
  type LogLine = { id: number; ts: string; level: string; msg: string };
  let logs = $state<LogLine[]>([]);
  let logId = 0;
  let logBox: HTMLDivElement;

  const LOG_TEMPLATES: { level: string; msg: string }[] = [
    { level: "INFO", msg: "Scanning frequencies 2.4GHz / 5GHz..." },
    { level: "OK", msg: "Connected to gateway 192.168.0.1" },
    { level: "INFO", msg: "Triangulating signal source vector..." },
    { level: "WARN", msg: "Jitter detected on channel 11" },
    { level: "OK", msg: "Handshake complete — link stable" },
    { level: "INFO", msg: "Sweeping sector for active beacons" },
    { level: "WARN", msg: "Packet loss spike: 2.1%" },
    { level: "INFO", msg: "Calibrating magnetometer offsets" },
    { level: "OK", msg: "GPS lock acquired (8 satellites)" },
    { level: "ERR", msg: "Beacon timeout — retrying probe" },
    { level: "INFO", msg: "Decoding SSID broadcast frame" },
    { level: "OK", msg: "Encryption negotiated: WPA3-SAE" },
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

  // Auto-scroll the terminal to the bottom whenever logs change.
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
    const SIZE = 260;
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
        life: 2200 + Math.random() * 1800,
      });
    }

    function draw(now: number) {
      ctx.clearRect(0, 0, SIZE, SIZE);

      // Grid rings
      ctx.strokeStyle = "rgba(125,207,255,0.18)";
      ctx.lineWidth = 1;
      for (let i = 1; i <= 4; i++) {
        ctx.beginPath();
        ctx.arc(c, c, (R / 4) * i, 0, Math.PI * 2);
        ctx.stroke();
      }
      // Cross-hairs
      ctx.beginPath();
      ctx.moveTo(c, c - R); ctx.lineTo(c, c + R);
      ctx.moveTo(c - R, c); ctx.lineTo(c + R, c);
      ctx.stroke();

      // Sweep gradient wedge
      const grad = ctx.createConicGradient(sweep - Math.PI / 2, c, c);
      grad.addColorStop(0, "rgba(158,206,106,0.0)");
      grad.addColorStop(0.92, "rgba(158,206,106,0.0)");
      grad.addColorStop(1, "rgba(158,206,106,0.35)");
      ctx.fillStyle = grad;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.arc(c, c, R, 0, Math.PI * 2);
      ctx.fill();

      // Sweep line
      ctx.strokeStyle = "rgba(158,206,106,0.9)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.lineTo(c + Math.cos(sweep - Math.PI / 2) * R, c + Math.sin(sweep - Math.PI / 2) * R);
      ctx.stroke();

      // Blips (fade over their lifetime)
      blips = blips.filter((b) => now - b.born < b.life);
      for (const b of blips) {
        const age = (now - b.born) / b.life;
        const alpha = Math.sin((1 - age) * Math.PI) * 0.9 + 0.1;
        const x = c + Math.cos(b.angle - Math.PI / 2) * b.radius;
        const y = c + Math.sin(b.angle - Math.PI / 2) * b.radius;
        ctx.fillStyle = `rgba(158,206,106,${alpha})`;
        ctx.shadowColor = "rgba(158,206,106,0.9)";
        ctx.shadowBlur = 8;
        ctx.beginPath();
        ctx.arc(x, y, 3, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;
      }

      sweep = (sweep + 0.02) % (Math.PI * 2);
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
    <span class="brand">◢ SIGNALUME</span>
    <span class="status"><span class="dot"></span> LIVE SCAN</span>
  </header>

  <!-- ── TOP: RADAR / COMPASS ───────────────────────────────────────── -->
  <section class="radar-wrap">
    <div class="radar-box">
      <canvas bind:this={canvas} class="radar" style="width:260px;height:260px;"></canvas>
      <div class="radar-center">
        <div class="heading">{Math.round(heading)}°</div>
        <div class="cardinal">{cardinalLong}</div>
        <div class="sensor-tag" class:live={usingRealSensor}>
          {usingRealSensor ? "● LIVE SENSOR" : "◌ SIMULATED"}
        </div>
      </div>
      <span class="tick n">N</span>
      <span class="tick e">E</span>
      <span class="tick s">S</span>
      <span class="tick w">W</span>
    </div>
  </section>

  <!-- ── MIDDLE: SIGNAL METRICS ─────────────────────────────────────── -->
  <section class="metrics">
    <div class="metric">
      <div class="metric-head">
        <span class="m-label cyan">Wi-Fi</span>
        <span class="m-val">{wifiDbm} dBm</span>
      </div>
      <div class="bar"><div class="fill cyan" style="width:{wifiPct}%"></div></div>
    </div>

    <div class="metric">
      <div class="metric-head">
        <span class="m-label purple">Cellular</span>
        <span class="m-val">{cellDbm} dBm</span>
      </div>
      <div class="bar"><div class="fill purple" style="width:{cellPct}%"></div></div>
    </div>

    <div class="mini-row">
      <div class="mini">
        <span class="mini-label">PING</span>
        <span class="mini-val green">{pingMs} <small>ms</small></span>
      </div>
      <div class="mini">
        <span class="mini-label">THROUGHPUT</span>
        <span class="mini-val cyan">{throughput} <small>Mb/s</small></span>
      </div>
      <div class="mini">
        <span class="mini-label">STATUS</span>
        <span class="mini-val green">ONLINE</span>
      </div>
    </div>
  </section>

  <!-- ── BOTTOM: TERMINAL LOG ───────────────────────────────────────── -->
  <section class="terminal">
    <div class="term-head">
      <span>SYSTEM LOG</span>
      <span class="term-dots"><i></i><i></i><i></i></span>
    </div>
    <div class="term-body" bind:this={logBox}>
      {#each logs as line (line.id)}
        <div class="log">
          <span class="ts">{line.ts}</span>
          <span class="lvl {line.level.toLowerCase()}">[{line.level}]</span>
          <span class="txt">{line.msg}</span>
        </div>
      {/each}
      <div class="cursor">▋</div>
    </div>
  </section>
</main>

<style>
  :global(html, body) {
    margin: 0;
    height: 100%;
    overflow: hidden; /* no global scrolling */
    background: #16161e;
  }

  .app {
    --bg: #1a1b26;
    --panel: #1f2335;
    --cyan: #7dcfff;
    --purple: #bb9af3;
    --green: #9ece6a;
    --red: #f7768e;
    --muted: #565f89;

    height: 100vh;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: 10px;
    padding: 12px;
    background: radial-gradient(circle at 50% 0%, #1f2335 0%, #16161e 70%);
    color: #c0caf5;
    font-family: "JetBrains Mono", "Fira Code", ui-monospace, "Courier New", monospace;
    overflow: hidden;
  }

  /* ── Header ── */
  .topbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 13px;
    letter-spacing: 2px;
  }
  .brand { color: var(--cyan); text-shadow: 0 0 8px rgba(125,207,255,0.6); }
  .status { color: var(--green); display: flex; align-items: center; gap: 6px; font-size: 11px; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--green); box-shadow: 0 0 8px var(--green);
    animation: pulse 1.2s infinite;
  }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.25; } }

  /* ── Radar ── */
  .radar-wrap { display: flex; justify-content: center; }
  .radar-box {
    position: relative;
    width: 260px; height: 260px;
    border-radius: 50%;
    background: rgba(13,14,22,0.6);
    box-shadow: 0 0 18px rgba(125,207,255,0.25), inset 0 0 30px rgba(0,0,0,0.6);
    border: 1px solid rgba(125,207,255,0.25);
  }
  .radar { display: block; }
  .radar-center {
    position: absolute; inset: 0;
    display: flex; flex-direction: column;
    align-items: center; justify-content: center;
    pointer-events: none;
  }
  .heading {
    font-size: 34px; font-weight: 700; color: var(--cyan);
    text-shadow: 0 0 12px rgba(125,207,255,0.8);
    line-height: 1;
  }
  .cardinal { font-size: 11px; letter-spacing: 3px; color: var(--green); margin-top: 4px; }
  .sensor-tag {
    margin-top: 6px; font-size: 8px; letter-spacing: 1.5px;
    color: var(--muted); opacity: 0.8;
  }
  .sensor-tag.live { color: var(--green); text-shadow: 0 0 6px var(--green); }
  .tick {
    position: absolute; font-size: 10px; color: var(--muted);
  }
  .tick.n { top: 4px; left: 50%; transform: translateX(-50%); }
  .tick.s { bottom: 4px; left: 50%; transform: translateX(-50%); }
  .tick.e { right: 6px; top: 50%; transform: translateY(-50%); }
  .tick.w { left: 6px; top: 50%; transform: translateY(-50%); }

  /* ── Metrics ── */
  .metrics {
    background: var(--panel);
    border: 1px solid rgba(125,207,255,0.12);
    border-radius: 10px;
    padding: 12px 14px;
    display: flex; flex-direction: column; gap: 12px;
  }
  .metric-head {
    display: flex; justify-content: space-between;
    font-size: 12px; margin-bottom: 5px;
  }
  .m-label { letter-spacing: 1px; }
  .m-val { color: #c0caf5; }
  .bar {
    height: 8px; border-radius: 4px;
    background: rgba(255,255,255,0.06); overflow: hidden;
  }
  .fill { height: 100%; border-radius: 4px; transition: width 0.6s ease; }
  .fill.cyan   { background: var(--cyan);   box-shadow: 0 0 10px var(--cyan); }
  .fill.purple { background: var(--purple); box-shadow: 0 0 10px var(--purple); }

  .cyan { color: var(--cyan); }
  .purple { color: var(--purple); }
  .green { color: var(--green); }

  .mini-row { display: flex; gap: 8px; }
  .mini {
    flex: 1;
    background: rgba(13,14,22,0.5);
    border: 1px solid rgba(125,207,255,0.1);
    border-radius: 8px;
    padding: 8px;
    display: flex; flex-direction: column; gap: 4px;
    align-items: center;
  }
  .mini-label { font-size: 9px; letter-spacing: 1px; color: var(--muted); }
  .mini-val { font-size: 15px; font-weight: 700; }
  .mini-val small { font-size: 9px; font-weight: 400; color: var(--muted); }

  /* ── Terminal ── */
  .terminal {
    flex: 1; min-height: 0;
    background: #0d0e16;
    border: 1px solid rgba(158,206,106,0.18);
    border-radius: 10px;
    display: flex; flex-direction: column;
    overflow: hidden;
  }
  .term-head {
    display: flex; justify-content: space-between; align-items: center;
    padding: 7px 12px;
    font-size: 10px; letter-spacing: 2px; color: var(--green);
    border-bottom: 1px solid rgba(158,206,106,0.15);
    background: rgba(158,206,106,0.05);
  }
  .term-dots { display: flex; gap: 5px; }
  .term-dots i { width: 8px; height: 8px; border-radius: 50%; background: rgba(255,255,255,0.15); }
  .term-dots i:nth-child(1) { background: var(--red); }
  .term-dots i:nth-child(2) { background: #e0af68; }
  .term-dots i:nth-child(3) { background: var(--green); }

  .term-body {
    flex: 1; min-height: 0;
    overflow-y: auto;
    padding: 8px 12px;
    font-size: 11.5px;
    line-height: 1.7;
    scrollbar-width: thin;
    scrollbar-color: #2a2e42 transparent;
  }
  .term-body::-webkit-scrollbar { width: 6px; }
  .term-body::-webkit-scrollbar-thumb { background: #2a2e42; border-radius: 3px; }

  .log { white-space: nowrap; }
  .ts { color: var(--muted); margin-right: 6px; }
  .lvl { margin-right: 6px; font-weight: 700; }
  .lvl.info { color: var(--cyan); }
  .lvl.ok   { color: var(--green); }
  .lvl.warn { color: #e0af68; }
  .lvl.err  { color: var(--red); }
  .txt { color: #a9b1d6; }
  .cursor { color: var(--green); animation: blink 1s steps(2) infinite; }
  @keyframes blink { 50% { opacity: 0; } }
</style>
