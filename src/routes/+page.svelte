<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  // Bundled (offline) type: condensed labels, plain body, mono readouts.
  import "@fontsource/barlow/latin-400.css";
  import "@fontsource/barlow/latin-500.css";
  import "@fontsource/barlow/latin-600.css";
  import "@fontsource/barlow-condensed/latin-500.css";
  import "@fontsource/barlow-condensed/latin-600.css";
  import "@fontsource/barlow-condensed/latin-700.css";
  import "@fontsource/jetbrains-mono/latin-400.css";
  import "@fontsource/jetbrains-mono/latin-500.css";
  import "@fontsource/jetbrains-mono/latin-700.css";

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
  //  Honesty note: on Android every number is measured (Wi-Fi / cellular dBm,
  //  link rate, ping). Only the desktop preview simulates a signal field, and
  //  it says so. Nothing in the UI pretends to scan hardware.
  // ════════════════════════════════════════════════════════════════════

  // ── Angle helpers (degrees, shortest signed path) ─────────────────────
  const angDiff = (a: number, b: number) => ((b - a + 540) % 360) - 180;
  const circLerp = (a: number, target: number, k: number) =>
    (a + angDiff(a, target) * k + 360) % 360;
  const clamp = (n: number, lo: number, hi: number) => Math.max(lo, Math.min(hi, n));

  // ── Themes ─────────────────────────────────────────────────────────────
  // Field-instrument palettes rather than generic "app" gradients. Each theme
  // is a flat var table applied to .app via a style string, so adding a theme
  // is a one-line edit and the canvas can read the same source of truth.
  type ThemeId =
    | "beacon" | "tide" | "phosphor" | "blueprint"
    | "ranger" | "crimson" | "paper" | "graphite";
  type Vars = {
    bg: string; surface: string; surface2: string; line: string;
    text: string; muted: string; accent: string;
    good: string; fair: string; weak: string;
  };
  type Theme = {
    label: string; blurb: string; group: "Signal" | "Field" | "Daylight";
    vars: Vars; accentRgb: string; inkRgb: string;
  };

  const THEMES: Record<ThemeId, Theme> = {
    beacon: {
      label: "Beacon", blurb: "Warm lighthouse amber on deep ink", group: "Signal",
      vars: { bg: "#14161b", surface: "#1b1e25", surface2: "#232730", line: "rgba(255,255,255,.06)",
        text: "#e7e9ee", muted: "#8b919e", accent: "#e0a04a", good: "#7fb88a", fair: "#d9b25e", weak: "#d98a6e" },
      accentRgb: "224,160,74", inkRgb: "255,255,255",
    },
    tide: {
      label: "Tide", blurb: "Cool, calm desaturated teal", group: "Signal",
      vars: { bg: "#101417", surface: "#171c20", surface2: "#1f262b", line: "rgba(255,255,255,.06)",
        text: "#e6ecef", muted: "#859098", accent: "#6fb3ad", good: "#7fb8a0", fair: "#d9b25e", weak: "#d98a6e" },
      accentRgb: "111,179,173", inkRgb: "255,255,255",
    },
    phosphor: {
      label: "Phosphor", blurb: "Old CRT / oscilloscope green", group: "Signal",
      vars: { bg: "#07100b", surface: "#0c1810", surface2: "#122418", line: "rgba(120,255,170,.09)",
        text: "#c7f3d6", muted: "#6f9a82", accent: "#46d67e", good: "#46d67e", fair: "#c9d96a", weak: "#e0905a" },
      accentRgb: "70,214,126", inkRgb: "199,243,214",
    },
    blueprint: {
      label: "Blueprint", blurb: "Cyan technical drawing on navy", group: "Field",
      vars: { bg: "#0a1622", surface: "#102234", surface2: "#16304a", line: "rgba(120,200,255,.10)",
        text: "#d3e7f5", muted: "#7793aa", accent: "#57b6e0", good: "#5fc0a8", fair: "#d6c06a", weak: "#e08a7a" },
      accentRgb: "87,182,224", inkRgb: "211,231,245",
    },
    ranger: {
      label: "Ranger", blurb: "Field-radio olive & signal amber", group: "Field",
      vars: { bg: "#14160d", surface: "#1c1f12", surface2: "#262a18", line: "rgba(220,210,150,.08)",
        text: "#e6e2cf", muted: "#9a9678", accent: "#c2a23a", good: "#8aa84f", fair: "#cda23a", weak: "#cc6f4a" },
      accentRgb: "194,162,58", inkRgb: "230,226,207",
    },
    crimson: {
      label: "Night Ops", blurb: "Red-on-black, preserves night vision", group: "Field",
      vars: { bg: "#130808", surface: "#1d0d0d", surface2: "#281212", line: "rgba(255,120,120,.09)",
        text: "#f0d2d0", muted: "#a07876", accent: "#e0564a", good: "#d49a86", fair: "#d97a55", weak: "#e64a3a" },
      accentRgb: "224,86,74", inkRgb: "240,210,208",
    },
    paper: {
      label: "Paper", blurb: "Daylight-readable field log", group: "Daylight",
      vars: { bg: "#ece8e1", surface: "#f5f2ea", surface2: "#e3ddd1", line: "rgba(0,0,0,.09)",
        text: "#2a2c30", muted: "#6b6f78", accent: "#c2722f", good: "#4f8f5e", fair: "#ab7f28", weak: "#b65a3e" },
      accentRgb: "194,114,47", inkRgb: "26,28,33",
    },
    graphite: {
      label: "Graphite", blurb: "Minimal brushed-steel monochrome", group: "Daylight",
      vars: { bg: "#16181c", surface: "#1d2024", surface2: "#262a30", line: "rgba(255,255,255,.07)",
        text: "#dfe3e8", muted: "#888f99", accent: "#8fa6bd", good: "#79b08c", fair: "#c2b06a", weak: "#c98a72" },
      accentRgb: "143,166,189", inkRgb: "223,227,232",
    },
  };
  const THEME_GROUPS = ["Signal", "Field", "Daylight"] as const;
  const themeVars = (id: ThemeId) => {
    const v = THEMES[id].vars;
    return `--bg:${v.bg};--surface:${v.surface};--surface-2:${v.surface2};--line:${v.line};` +
      `--text:${v.text};--muted:${v.muted};--accent:${v.accent};` +
      `--good:${v.good};--fair:${v.fair};--weak:${v.weak};`;
  };

  // ── Settings (persisted) ──────────────────────────────────────────────
  type SmoothId = "responsive" | "balanced" | "steady";
  type Settings = {
    theme: ThemeId;
    smoothing: SmoothId;
    // Behaviour
    haptics: boolean;       // vibrate when saving a reading
    buzzOnLock: boolean;    // vibrate the moment you face the strongest signal
    keepAwake: boolean;     // hold the screen on while hunting
    // Display — let people hide whatever they don't want on screen
    showHint: boolean;      // the helper line under the Save button
    showGraticule: boolean; // faint rings + crosshairs on the dial
    showReadout: boolean;   // the "%· 184° N" line in the dial centre
  };
  const DEFAULTS: Settings = {
    theme: "beacon", smoothing: "balanced",
    haptics: true, buzzOnLock: true, keepAwake: false,
    showHint: true,
    showGraticule: true, showReadout: true,
  };
  let settings = $state<Settings>({ ...DEFAULTS });
  let aboutOpen = $state(false);
  let ready = $state(false); // gate persistence until stored values are loaded

  // Bottom-tab navigation. Every screen stays mounted (hidden, not removed) so
  // the dial canvas and its animation loop survive tab switches.
  type TabId = "find" | "spots" | "signal" | "settings";
  let tab = $state<TabId>("find");
  function goTab(t: TabId) {
    if (t === tab) return;
    tab = t;
    window.scrollTo({ top: 0 });
  }

  // Canvas can't read CSS vars cheaply per frame, so mirror the colours it
  // needs as plain RGB triplets: the accent (needle) and the "ink" used for
  // rings / ticks / labels.
  let accentRgb = $derived(THEMES[settings.theme].accentRgb);
  let inkRgb = $derived(THEMES[settings.theme].inkRgb);

  // How hard the on-screen heading eases toward the sensor. Lower = steadier.
  const SMOOTH: Record<SmoothId, number> = {
    responsive: 0.22, balanced: 0.12, steady: 0.06,
  };

  $effect(() => {
    if (!ready) return;
    try { localStorage.setItem("signalume.settings", JSON.stringify(settings)); } catch {}
  });

  // Keep the page background in sync with the theme (covers overscroll areas).
  $effect(() => {
    if (typeof document !== "undefined")
      document.body.style.background = THEMES[settings.theme].vars.bg;
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

  // ── Signal field (real Wi-Fi RSSI when available, else simulated) ──────
  let bestBearing = 47;            // "strongest direction" — measured (real) or drifting (sim)
  let qNoise = $state(0);          // small smoothed measurement noise (sim only)

  // Signal source state, fed by the native `signal_reading` command on Android.
  //   "demo" — desktop/browser preview only (simulated field, clearly labelled)
  //   "wifi" — connected to Wi-Fi, showing live RSSI
  //   "cell" — no Wi-Fi but mobile data active, showing cellular dBm
  //   "none" — on a device but no signal at all: honest empty state, never fake.
  let signalMode = $state<"demo" | "wifi" | "cell" | "none">("demo");
  let usingRealSignal = $derived(signalMode === "wifi" || signalMode === "cell");
  let noSignal = $derived(signalMode === "none");
  let rssiDbm = $state(0);   // active dBm (wifi or cell, whichever is current)
  let cellRssiDbm = $state(0); // raw cell dBm even when wifi is primary
  let wifiLinkMbps = $state(0);  // negotiated Wi-Fi link rate, 0 = unknown
  let locationGranted = $state(true); // cellular needs runtime location permission
  let livePingMs = $state<number | null>(null); // measured TCP handshake RTT
  // Wi-Fi RSSI → 0..100: −90 dBm → unusable, −40 dBm → excellent.
  const rssiToQuality = (dbm: number) => clamp(Math.round((dbm + 90) * 2), 2, 99);
  // Cellular RSRP → 0..100: −120 dBm → unusable, −80 dBm → excellent.
  const cellToQuality = (dbm: number) => clamp(Math.round((dbm + 120) * 2.5), 2, 99);
  // Best RSSI measured per 15° heading bucket — this is how the arrow learns
  // which real direction is strongest as you turn.
  let realSamples = new Map<number, number>();

  // Raw quality 0..100. Real mode reads the live RSSI; sim mode peaks when the
  // phone faces bestBearing and dips facing away.
  let qualityRaw = $derived.by(() => {
    if (signalMode === "wifi") return rssiToQuality(rssiDbm);
    if (signalMode === "cell") return cellToQuality(rssiDbm);
    if (signalMode === "none") return 0;
    return clamp(Math.round(
      50 + 47 * Math.cos((angDiff(heading, bestBearing) * Math.PI) / 180) + qNoise
    ), 2, 99);
  });
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

  // Technical readouts. On a device every number is measured — anything we
  // can't measure shows "—" rather than an estimate. Demo mode alone fills in
  // simulated values (and the card is labelled "simulated").
  let isDemo = $derived(signalMode === "demo");
  let wifiDbm = $derived<number | null>(
    signalMode === "wifi" ? rssiDbm : isDemo ? Math.round(-92 + quality * 0.5) : null
  );
  let cellDbm = $derived<number | null>(
    cellRssiDbm < 0 ? cellRssiDbm : isDemo ? Math.round(-100 + quality * 0.4) : null
  );
  let wifiPct = $derived(wifiDbm == null ? 0 : isDemo ? quality : rssiToQuality(wifiDbm));
  let cellPct = $derived(cellDbm == null ? 0 : isDemo ? clamp(quality - 14, 0, 100) : cellToQuality(cellDbm));
  let pingMs = $derived<number | null>(isDemo ? Math.round(58 - quality * 0.46) : livePingMs);
  let linkMbps = $derived<number | null>(
    isDemo ? Math.round(8 + quality * 0.85) : signalMode === "wifi" && wifiLinkMbps > 0 ? wifiLinkMbps : null
  );
  let detailsTag = $derived(
    signalMode === "wifi" ? "live Wi-Fi" : signalMode === "cell" ? "live cellular" :
    noSignal ? "no signal" : "simulated"
  );

  // ── Saved spots ("readings") ──────────────────────────────────────────
  type Pin = {
    id: number; bearing: number; quality: number; point: string;
    src?: "wifi" | "cell" | "demo"; // what was measured (older saves lack it)
    t?: number;                       // when it was saved (epoch ms)
  };
  const PIN_SRC_LABEL = { wifi: "Wi-Fi", cell: "Cell", demo: "Demo" } as const;
  let pins = $state<Pin[]>([]);
  let pinId = 0;

  $effect(() => {
    if (!ready) return;
    try { localStorage.setItem("signalume.pins", JSON.stringify(pins)); } catch {}
  });

  function takeReading() {
    if (noSignal) return; // nothing meaningful to pin
    haptic([12, 28, 12]);
    const b = Math.round(heading);
    const point = COMPASS_POINTS[Math.round(b / 22.5) % 16];
    pins = [...pins.slice(-15), { id: pinId++, bearing: b, quality, point, src: signalMode as Pin["src"], t: Date.now() }];
    showToast(`Spot saved · ${quality}% facing ${point}`);
  }
  let bestPin = $derived.by(() =>
    pins.reduce<Pin | null>((b, p) => (!b || p.quality > b.quality ? p : b), null)
  );
  let sortedPins = $derived([...pins].sort((a, b) => b.quality - a.quality));
  function clearPins() { pins = []; showToast("Saved spots cleared"); }
  function deletePin(id: number) { pins = pins.filter((p) => p.id !== id); }
  function pinTime(t?: number) {
    if (!t) return "";
    const d = new Date(t);
    const hm = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
    return d.toDateString() === new Date().toDateString()
      ? hm : `${d.toLocaleDateString([], { day: "numeric", month: "short" })} ${hm}`;
  }

  // ── Signal history (Signal tab sparkline) ─────────────────────────────
  // One sample per second; null marks seconds with no signal so the line
  // breaks instead of drawing a fake bridge across the gap.
  const HISTORY_LEN = 120;
  let history = $state<(number | null)[]>([]);
  let histStats = $derived.by(() => {
    const v = history.filter((x): x is number => x != null);
    if (!v.length) return null;
    return {
      min: Math.min(...v), max: Math.max(...v),
      avg: Math.round(v.reduce((a, b) => a + b, 0) / v.length),
    };
  });
  const SPARK_W = 300, SPARK_H = 90;
  let sparkPath = $derived.by(() => {
    let d = "", pen = false;
    history.forEach((q, i) => {
      if (q == null) { pen = false; return; }
      const x = (i / (HISTORY_LEN - 1)) * SPARK_W;
      const y = SPARK_H - (q / 100) * SPARK_H;
      d += `${pen ? "L" : "M"}${x.toFixed(1)},${y.toFixed(1)}`;
      pen = true;
    });
    return d;
  });

  function qualityClass(q: number) {
    return q >= 68 ? "good" : q >= 45 ? "fair" : "weak";
  }

  // ── Lightweight toast ──────────────────────────────────────────────────
  let toast = $state<string | null>(null);
  let toastTimer: any;
  function showToast(msg: string) {
    toast = msg;
    clearTimeout(toastTimer);
    toastTimer = setTimeout(() => (toast = null), 1900);
  }

  // ── Dial canvas ───────────────────────────────────────────────────────
  let canvas: HTMLCanvasElement;
  let dialWrap: HTMLDivElement;

  onMount(() => {
    // Load saved settings before enabling persistence.
    try {
      const raw = localStorage.getItem("signalume.settings");
      if (raw) settings = { ...DEFAULTS, ...JSON.parse(raw) };
    } catch {}
    if (!THEMES[settings.theme]) settings.theme = "beacon"; // migrate old ids
    try {
      const raw = localStorage.getItem("signalume.pins");
      if (raw) {
        const saved: Pin[] = JSON.parse(raw);
        if (Array.isArray(saved)) {
          pins = saved;
          pinId = saved.reduce((m, p) => Math.max(m, p.id + 1), 0);
        }
      }
    } catch {}
    ready = true;

    // The dial is drawn in a fixed 236-unit space and scaled to whatever size
    // the layout gives it, so it fills small and large phones alike.
    const ctx = canvas.getContext("2d")!;
    const SIZE = 236;
    function fitCanvas() {
      const css = dialWrap.clientWidth;
      if (!css) return; // hidden tab
      const DPR = window.devicePixelRatio || 1;
      canvas.width = Math.round(css * DPR);
      canvas.height = Math.round(css * DPR);
      const k = (css / SIZE) * DPR;
      ctx.setTransform(k, 0, 0, k, 0, 0);
    }
    fitCanvas();
    const ro = new ResizeObserver(fitCanvas);
    ro.observe(dialWrap);
    const c = SIZE / 2;
    const R = c - 10;      // outer bezel ring (room above for the lubber mark)
    const RI = R - 30;     // inner ring: pins and the peak pointer live here

    // Convert a bearing-relative-to-heading into a canvas point (up = aligned).
    const posFor = (bearingRel: number, r: number): [number, number] => {
      const a = ((bearingRel - 90) * Math.PI) / 180;
      return [c + Math.cos(a) * r, c + Math.sin(a) * r];
    };
    const rad = (bearingRel: number) => ((bearingRel - 90) * Math.PI) / 180;
    const line = (a: [number, number], b: [number, number]) => {
      ctx.beginPath(); ctx.moveTo(a[0], a[1]); ctx.lineTo(b[0], b[1]); ctx.stroke();
    };
    // Aviation compass-card labels: cardinals as letters, the rest in tens.
    const CARD: Record<number, string> = { 0: "N", 90: "E", 180: "S", 270: "W" };
    const MONO = "'JetBrains Mono', ui-monospace, monospace";

    let raf = 0;
    function draw() {
      if (tab !== "find") { raf = requestAnimationFrame(draw); return; }
      const ac = accentRgb;
      const ink = inkRgb;
      const tv = THEMES[settings.theme].vars;
      ctx.clearRect(0, 0, SIZE, SIZE);
      ctx.lineCap = "butt";
      ctx.setLineDash([]);

      // Bezel ring + a tick every 5°, longer every 15° / 30°. Rotates with heading.
      ctx.strokeStyle = `rgba(${ink},0.24)`;
      ctx.lineWidth = 1;
      ctx.beginPath(); ctx.arc(c, c, R, 0, Math.PI * 2); ctx.stroke();
      for (let deg = 0; deg < 360; deg += 5) {
        const major = deg % 30 === 0, mid = deg % 15 === 0;
        const len = major ? 9 : mid ? 6 : 3;
        ctx.strokeStyle = `rgba(${ink},${major ? 0.6 : mid ? 0.32 : 0.16})`;
        ctx.lineWidth = major ? 1.4 : 1;
        line(posFor(deg - heading, R - 2), posFor(deg - heading, R - 2 - len));
      }

      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (let deg = 0; deg < 360; deg += 30) {
        const card = CARD[deg];
        ctx.font = card ? `700 10px ${MONO}` : `500 7.5px ${MONO}`;
        ctx.fillStyle = deg === 0 ? `rgba(${ac},1)` : `rgba(${ink},${card ? 0.8 : 0.42})`;
        const [lx, ly] = posFor(deg - heading, R - 19);
        ctx.fillText(card ?? String(deg / 10), lx, ly);
      }

      // Inner ring.
      ctx.strokeStyle = `rgba(${ink},0.14)`;
      ctx.lineWidth = 1;
      ctx.beginPath(); ctx.arc(c, c, RI, 0, Math.PI * 2); ctx.stroke();

      // Optional graticule: dashed range rings + a ticked crosshair.
      if (settings.showGraticule) {
        ctx.strokeStyle = `rgba(${ink},0.08)`;
        ctx.setLineDash([2, 3]);
        for (const k of [1 / 3, 2 / 3]) {
          ctx.beginPath(); ctx.arc(c, c, RI * k, 0, Math.PI * 2); ctx.stroke();
        }
        ctx.setLineDash([]);
        ctx.strokeStyle = `rgba(${ink},0.06)`;
        line([c, c - RI], [c, c + RI]);
        line([c - RI, c], [c + RI, c]);
        for (let i = -3; i <= 3; i++) {
          if (!i) continue;
          const o = (RI / 4) * i;
          line([c + o, c - 2], [c + o, c + 2]);
          line([c - 2, c + o], [c + 2, c + o]);
        }
      }

      // Fixed lubber mark: where the phone is pointing. Never rotates.
      ctx.fillStyle = `rgba(${ink},0.92)`;
      ctx.beginPath();
      ctx.moveTo(c - 5, 0); ctx.lineTo(c + 5, 0); ctx.lineTo(c, c - R - 1);
      ctx.closePath(); ctx.fill();

      // "Aligned" window: brackets at ±12° on the inner ring.
      ctx.strokeStyle = `rgba(${ac},0.55)`;
      ctx.lineWidth = 1.5;
      for (const s of [-12, 12]) line(posFor(s, RI - 6), posFor(s, RI + 6));
      ctx.strokeStyle = `rgba(${ac},0.25)`;
      ctx.beginPath(); ctx.arc(c, c, RI, rad(-12), rad(12)); ctx.stroke();

      // Saved spots as diamonds just inside the inner ring.
      for (const p of pins) {
        const [x, y] = posFor(p.bearing - heading, RI - 10);
        const isBest = bestPin && p.id === bestPin.id;
        const col = p.quality >= 68 ? tv.good : p.quality >= 45 ? tv.fair : tv.weak;
        const s = isBest ? 3.6 : 2.4;
        ctx.save();
        ctx.translate(x, y); ctx.rotate(Math.PI / 4);
        ctx.globalAlpha = isBest ? 1 : 0.6;
        ctx.fillStyle = col;
        ctx.fillRect(-s, -s, s * 2, s * 2);
        if (isBest) {
          ctx.strokeStyle = col; ctx.lineWidth = 1;
          ctx.strokeRect(-s - 3, -s - 3, (s + 3) * 2, (s + 3) * 2);
        }
        ctx.restore();
      }

      // Peak pointer: a shaded sector across the bezel plus a hard wedge that
      // points outward at the strongest direction. Hidden with no signal.
      if (signalMode !== "none") {
        const rel = bestBearing - heading;
        ctx.fillStyle = `rgba(${ac},0.14)`;
        ctx.beginPath();
        ctx.arc(c, c, R - 1, rad(rel - 10), rad(rel + 10));
        ctx.arc(c, c, RI, rad(rel + 10), rad(rel - 10), true);
        ctx.closePath(); ctx.fill();
        ctx.strokeStyle = `rgba(${ac},0.5)`;
        ctx.lineWidth = 1;
        line(posFor(rel - 10, RI), posFor(rel - 10, R - 1));
        line(posFor(rel + 10, RI), posFor(rel + 10, R - 1));

        const tip = posFor(rel, R - 3);
        const l = posFor(rel - 4.5, RI + 1);
        const r = posFor(rel + 4.5, RI + 1);
        ctx.fillStyle = `rgba(${ac},1)`;
        ctx.beginPath();
        ctx.moveTo(tip[0], tip[1]); ctx.lineTo(l[0], l[1]); ctx.lineTo(r[0], r[1]);
        ctx.closePath(); ctx.fill();
        ctx.strokeStyle = `rgba(${ac},0.9)`;
        ctx.lineWidth = 2;
        line(posFor(rel, RI - 4), posFor(rel, RI - 16));
      }

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

    // The strongest direction drifts slowly so the app stays alive (demo only).
    const driftTimer = setInterval(() => {
      if (signalMode !== "demo") return;
      bestBearing = (bestBearing + (Math.random() - 0.5) * 6 + 360) % 360;
    }, 7000);

    // Poll native Wi-Fi + cellular RSSI in one call (Android only). Priority:
    // Wi-Fi → cell → none. As the phone turns we track the peak RSSI per 15°
    // bucket and point the arrow at the direction that measured strongest.
    // Desktop Tauri falls back to the labelled demo mode.
    let signalPoll: any = 0;
    let pingPoll: any = 0;
    let askedLocation = false;
    const hasNative = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    signalMode = hasNative ? "none" : "demo";

    function trackBearing(rssi: number) {
      const bucket = (Math.round(heading / 15) * 15 + 360) % 360;
      const prev = realSamples.get(bucket);
      realSamples.set(bucket, prev == null ? rssi : Math.max(prev - 1, rssi));
      let bb = bucket, bv = -999;
      for (const [k, v] of realSamples) if (v > bv) { bv = v; bb = k; }
      bestBearing = bb;
    }

    const pollSignal = async () => {
      try {
        const r = await invoke<{
          wifi_rssi: number; wifi_connected: boolean; wifi_link_mbps: number;
          cell_rssi: number; cell_connected: boolean; location_granted: boolean;
        }>("signal_reading");

        locationGranted = r.location_granted;
        // Ask once per launch; Android itself stops showing the dialog after
        // repeated denials, so nagging every poll would be pointless.
        if (!r.location_granted && !askedLocation) {
          askedLocation = true;
          invoke("request_location_permission").catch(() => {});
        }
        wifiLinkMbps = r.wifi_connected ? r.wifi_link_mbps : 0;

        if (r.wifi_connected) {
          if (signalMode !== "wifi") realSamples.clear();
          rssiDbm = r.wifi_rssi;
          cellRssiDbm = r.cell_connected ? r.cell_rssi : 0;
          signalMode = "wifi";
          trackBearing(r.wifi_rssi);
        } else if (r.cell_connected) {
          if (signalMode !== "cell") realSamples.clear();
          rssiDbm = r.cell_rssi;
          cellRssiDbm = r.cell_rssi;
          signalMode = "cell";
          trackBearing(r.cell_rssi);
        } else {
          cellRssiDbm = 0;
          signalMode = "none";
        }
      } catch (e) {
        if (String(e).includes("only available on Android")) {
          signalMode = "demo";
          if (signalPoll) { clearInterval(signalPoll); signalPoll = 0; }
          if (pingPoll) { clearInterval(pingPoll); pingPoll = 0; }
        } else {
          signalMode = "none";
        }
      }
    };
    // Latency is a real TCP handshake, so poll it gently (it costs a packet).
    const pollPing = async () => {
      try { livePingMs = await invoke<number | null>("ping_ms"); }
      catch { livePingMs = null; }
    };
    if (hasNative) {
      pollSignal(); signalPoll = setInterval(pollSignal, 700);
      pollPing(); pingPoll = setInterval(pollPing, 4000);
    }

    const historyTimer = setInterval(() => {
      history = [...history.slice(-(HISTORY_LEN - 1)), noSignal ? null : quality];
    }, 1000);

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
      clearInterval(historyTimer);
      ro.disconnect();
      if (signalPoll) clearInterval(signalPoll);
      if (pingPoll) clearInterval(pingPoll);
      clearTimeout(toastTimer);
    };
  });
</script>


<main class="app" data-theme={settings.theme} style={themeVars(settings.theme)}>
  <!-- ── HEADER ──────────────────────────────────────────────────────── -->
  <header class="topbar">
    <span class="brand">
      <img class="logo" src="/logo.svg" alt="" />
      <span class="brand-txt">
        <b>Signalume</b>
        <small>Field unit · SL-01</small>
      </span>
    </span>
    <button class="src" class:live={usingRealSignal} class:warn={noSignal}
            onclick={() => (aboutOpen = true)} aria-label="Signal source — how it works">
      <span class="led"></span>
      <span class="src-k">SRC</span>
      <span class="src-v">
        {#if signalMode === "wifi"}Wi-Fi
        {:else if signalMode === "cell"}Cell
        {:else if noSignal}None
        {:else}Demo{/if}
      </span>
    </button>
  </header>

  <!-- ══ FIND ═══════════════════════════════════════════════════════════ -->
  <section class="screen find" hidden={tab !== "find"}>
    <div class="sec"><span class="sec-n">01</span><span class="sec-t">Direction finder</span><span class="sec-r"></span></div>

    <div class="telemetry">
      <div class:live={usingRealSignal}>
        <small>{signalMode === "cell" ? "RSRP" : "RSSI"}</small>
        <b>{#if usingRealSignal}{rssiDbm}<i>dBm</i>{:else if noSignal}—{:else}SIM{/if}</b>
      </div>
      <div>
        <small>HDG</small>
        <b>{String(Math.round(shownHeading) % 360).padStart(3, "0")}°<i>{cardinal}</i></b>
      </div>
      <div class:live={usingRealSensor}>
        <small>CMP</small>
        <b>{usingRealSensor ? "LIVE" : "OFF"}</b>
      </div>
    </div>

    <div class="dial-wrap" bind:this={dialWrap}>
      <canvas bind:this={canvas} class="dial"></canvas>
      <div class="dial-center">
        <div class="q-cap">SIG</div>
        {#if noSignal}
          <div class="q-big none">--</div>
          <div class="q-label">No signal</div>
        {:else}
          <div class="q-big {qInfo.cls}">{String(quality).padStart(2, "0")}<span>%</span></div>
          <div class="q-label {qInfo.cls}">{qInfo.label}</div>
          <div class="segs {qInfo.cls}" aria-hidden="true">
            {#each Array(10) as _, i}
              <span class="seg" class:on={i < Math.ceil(quality / 10)}></span>
            {/each}
          </div>
        {/if}
        {#if settings.showReadout}
          <div class="q-head">{Math.round(shownHeading)}° {cardinal}</div>
        {/if}
      </div>
    </div>

    {#if noSignal}
      <div class="guide warn">
        <span class="guide-ico">!</span>
        <span class="guide-text">
          <b>No signal source</b>
          <small>
            {#if locationGranted}
              Connect to Wi-Fi or turn on mobile data
            {:else}
              Connect to Wi-Fi, or allow Location in Android settings to read mobile signal
            {/if}
          </small>
        </span>
      </div>
    {:else}
      <div class="guide" class:aligned={guidance.aligned}>
        <span class="guide-ico">
          {#if guidance.aligned}
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4.5 12.5l5 5L19.5 7" /></svg>
          {:else}
            <svg viewBox="0 0 24 24" aria-hidden="true" style="transform: rotate({clamp(toBest, -150, 150)}deg)">
              <path d="M12 21V4M5 11l7-7 7 7" />
            </svg>
          {/if}
        </span>
        <span class="guide-text">
          <b>{guidance.aligned ? "Locked on" : guidance.main}</b>
          <small>{guidance.sub}</small>
        </span>
        <span class="guide-deg">{toBest > 0 ? "+" : toBest < 0 ? "−" : ""}{Math.abs(Math.round(toBest))}°</span>
      </div>
    {/if}

    <div class="find-actions">
      <button class="primary-btn" onclick={takeReading} disabled={noSignal}>
        {#if noSignal}Waiting for signal…{:else}
          <span class="pb-l">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v4M12 17v4M3 12h4M17 12h4" /><rect x="9" y="9" width="6" height="6" /></svg>
            Mark this spot
          </span>
          <span class="pb-r">{quality}% · {cardinal}</span>
        {/if}
      </button>
      {#if settings.showHint}
        <p class="hint">Turn slowly in a full circle — the pointer learns where the signal peaks.</p>
      {/if}
    </div>
  </section>

  <!-- ══ SPOTS ══════════════════════════════════════════════════════════ -->
  <section class="screen" hidden={tab !== "spots"}>
    <div class="sec"><span class="sec-n">02</span><span class="sec-t">Survey log</span><span class="sec-r"></span><span class="sec-x">{pins.length} fix{pins.length === 1 ? "" : "es"}</span></div>
    {#if !pins.length}
      <div class="panel empty-state">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 3v4M12 17v4M3 12h4M17 12h4" /><rect x="9" y="9" width="6" height="6" /></svg>
        <b>Log is empty</b>
        <p>Walk to a few places and tap <b>Mark this spot</b> on the Find tab. The strongest fix gets a ★.</p>
        <button class="ghost-btn" onclick={() => goTab("find")}>Open finder</button>
      </div>
    {:else}
      {#if bestPin}
        <div class="panel best-card">
          <div class="p-head"><span>★ Best fix</span><span>{bestPin.src ? PIN_SRC_LABEL[bestPin.src] : ""}{bestPin.src && bestPin.t ? " · " : ""}{pinTime(bestPin.t)}</span></div>
          <div class="best-row">
            <span class="best-q {qualityClass(bestPin.quality)}">{bestPin.quality}<small>%</small></span>
            <span class="best-info">
              <small>Bearing</small>
              <b>{String(bestPin.bearing % 360).padStart(3, "0")}° {bestPin.point}</b>
            </span>
          </div>
        </div>
      {/if}
      <div class="panel list-card">
        <div class="p-head"><span># · Bearing · Signal</span><button class="link" onclick={clearPins}>Clear all</button></div>
        {#each sortedPins as p, i (p.id)}
          <div class="spot" class:best={bestPin && p.id === bestPin.id}>
            <span class="spot-rank">{String(i + 1).padStart(2, "0")}</span>
            <span class="spot-main">
              <span class="spot-dir">{String(p.bearing % 360).padStart(3, "0")}° {p.point}</span>
              <span class="spot-sub">{p.src ? PIN_SRC_LABEL[p.src] : ""}{p.src && p.t ? " · " : ""}{pinTime(p.t)}</span>
              <span class="bar sm"><span class="fill {qualityClass(p.quality)}" style="width:{p.quality}%"></span></span>
            </span>
            <span class="spot-q {qualityClass(p.quality)}">{p.quality}%</span>
            <button class="spot-del" aria-label="Delete spot" onclick={() => deletePin(p.id)}>
              <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M7 7l10 10M17 7L7 17" /></svg>
            </button>
          </div>
        {/each}
      </div>
    {/if}
  </section>

  <!-- ══ SIGNAL ═════════════════════════════════════════════════════════ -->
  <section class="screen" hidden={tab !== "signal"}>
    <div class="sec"><span class="sec-n">03</span><span class="sec-t">Signal monitor</span><span class="sec-r"></span></div>
    <div class="panel hero">
      <div class="p-head">
        <span class="sim-tag" class:live={usingRealSignal} class:warn={noSignal}><span class="led"></span>{detailsTag}</span>
        <span>
          {#if signalMode === "wifi"}Wi-Fi{:else if signalMode === "cell"}Mobile data{:else if noSignal}No source{:else}Demo{/if}
        </span>
      </div>
      <div class="hero-body">
        <div class="hero-val">
          {#if usingRealSignal}{rssiDbm}<small>dBm</small>
          {:else if noSignal}--
          {:else}{quality}<small>%</small>{/if}
        </div>
        <div class="hero-sub {noSignal ? '' : qInfo.cls}">{noSignal ? "Waiting for a signal" : qInfo.label}</div>

        <div class="spark-box">
          <svg class="spark" viewBox="0 0 {SPARK_W} {SPARK_H}" preserveAspectRatio="none" aria-label="Signal over the last two minutes">
            {#each [25, 50, 75] as g}
              <line x1="0" x2={SPARK_W} y1={SPARK_H - (g / 100) * SPARK_H} y2={SPARK_H - (g / 100) * SPARK_H} class="grid" />
            {/each}
            {#each [1, 2, 3, 4, 5, 6, 7] as g}
              <line y1="0" y2={SPARK_H} x1={(g / 8) * SPARK_W} x2={(g / 8) * SPARK_W} class="grid v" />
            {/each}
            {#if sparkPath}<path d={sparkPath} class="line" />{/if}
          </svg>
          <span class="spark-y top">100</span><span class="spark-y mid">50</span>
        </div>
        <div class="spark-axis"><span>−2:00</span><span>−1:00</span><span>Now</span></div>
      </div>

      <div class="stats">
        <div><small>Low</small><b>{histStats ? histStats.min : "--"}<i>%</i></b></div>
        <div><small>Avg</small><b>{histStats ? histStats.avg : "--"}<i>%</i></b></div>
        <div><small>High</small><b>{histStats ? histStats.max : "--"}<i>%</i></b></div>
      </div>
    </div>

    <div class="panel det-body">
      <div class="p-head"><span>Channels</span><span>dBm</span></div>
      <div class="rows">
        <div class="row">
          <span class="r-label">Wi-Fi</span>
          <span class="bar ticks"><span class="fill" style="width:{wifiPct}%"></span></span>
          <span class="r-val">{wifiDbm != null ? wifiDbm : "--"}</span>
        </div>
        <div class="row">
          <span class="r-label">Cell</span>
          <span class="bar ticks"><span class="fill" style="width:{cellPct}%"></span></span>
          <span class="r-val">{cellDbm != null ? cellDbm : "--"}</span>
        </div>
      </div>
      <div class="mini-row">
        <div class="mini"><span class="mini-l">Ping</span><span class="mini-v">{pingMs != null ? pingMs : "--"}<small>ms</small></span></div>
        <div class="mini"><span class="mini-l">Link</span><span class="mini-v">{linkMbps != null ? linkMbps : "--"}<small>Mb/s</small></span></div>
        <div class="mini"><span class="mini-l">Hdg</span><span class="mini-v">{Math.round(shownHeading)}<small>°</small></span></div>
      </div>
      {#if !locationGranted}
        <p class="note">Mobile signal needs the <b>Location</b> permission. Allow it in Android settings → Apps → Signalume.</p>
      {/if}
    </div>
  </section>

  <!-- ══ SETTINGS ═══════════════════════════════════════════════════════ -->
  <section class="screen" hidden={tab !== "settings"}>
    <div class="sec"><span class="sec-n">04</span><span class="sec-t">Configuration</span><span class="sec-r"></span></div>

    <div class="set-group">
      <div class="set-title"><span>A</span>Theme</div>
      {#each THEME_GROUPS as group}
        <div class="theme-group-label">{group}</div>
        <div class="themes">
          {#each Object.entries(THEMES).filter(([, t]) => t.group === group) as [id, t]}
            <button class="theme-chip" class:on={settings.theme === id}
                    onclick={() => (settings.theme = id as ThemeId)} title={t.blurb}>
              <span class="sw-preview" style={themeVars(id as ThemeId)}>
                <span class="sw-bg"></span><span class="sw-accent"></span>
              </span>
              <span class="sw-label">{t.label}</span>
            </button>
          {/each}
        </div>
      {/each}
      <p class="set-note">{THEMES[settings.theme].blurb}.</p>
    </div>

    <div class="set-group">
      <div class="set-title"><span>B</span>Compass steadiness</div>
      <div class="seg-pick">
        {#each [["responsive","Responsive"],["balanced","Balanced"],["steady","Steady"]] as [id, label]}
          <button class="seg-opt" class:on={settings.smoothing === id}
                  onclick={() => (settings.smoothing = id as SmoothId)}>{label}</button>
        {/each}
      </div>
      <p class="set-note">Pick <b>Steady</b> if the numbers jump around too much.</p>
    </div>

    <div class="set-group">
      <div class="set-title"><span>C</span>Feedback</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (settings.buzzOnLock = !settings.buzzOnLock)}>
          <span><b>Buzz when aligned</b><small>Vibrate the instant you face the strongest signal</small></span>
          <span class="sw-track" class:on={settings.buzzOnLock}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.haptics = !settings.haptics)}>
          <span><b>Haptic feedback</b><small>Vibrate when marking a spot</small></span>
          <span class="sw-track" class:on={settings.haptics}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.keepAwake = !settings.keepAwake)}>
          <span><b>Keep screen on</b><small>Stop the display sleeping while hunting</small></span>
          <span class="sw-track" class:on={settings.keepAwake}><span class="knob"></span></span>
        </button>
      </div>
    </div>

    <div class="set-group">
      <div class="set-title"><span>D</span>Dial</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (settings.showReadout = !settings.showReadout)}>
          <span><b>Heading readout</b><small>The “184° S” line inside the dial</small></span>
          <span class="sw-track" class:on={settings.showReadout}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.showGraticule = !settings.showGraticule)}>
          <span><b>Graticule</b><small>Range rings and crosshair behind the pointer</small></span>
          <span class="sw-track" class:on={settings.showGraticule}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.showHint = !settings.showHint)}>
          <span><b>Helper text</b><small>The short tip under the Mark button</small></span>
          <span class="sw-track" class:on={settings.showHint}><span class="knob"></span></span>
        </button>
      </div>
    </div>

    <div class="set-group">
      <div class="set-title"><span>E</span>Data</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (aboutOpen = true)}>
          <span><b>How the scan works</b><small>What's measured and what isn't</small></span>
          <span class="chev">
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M9 5l7 7-7 7" /></svg>
          </span>
        </button>
      </div>
      <div class="danger-row">
        <button class="danger" onclick={clearPins} disabled={!pins.length}>Clear log ({pins.length})</button>
        <button class="danger soft" onclick={() => { settings = { ...DEFAULTS }; showToast("Settings reset"); }}>
          Reset settings
        </button>
      </div>
    </div>

    <div class="ver"><span>Signalume</span><span>v0.1 · SL-01</span></div>
  </section>

  <!-- ── TAB BAR ─────────────────────────────────────────────────────── -->
  <nav class="tabbar" aria-label="Sections">
    <button class:on={tab === "find"} onclick={() => goTab("find")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8.5" /><path d="M12 3.5v3M12 17.5v3M3.5 12h3M17.5 12h3M12 8l2.5 6.5L12 13l-2.5 1.5z" /></svg>
      <span>Find</span>
    </button>
    <button class:on={tab === "spots"} onclick={() => goTab("spots")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h10" /></svg>
      <span>Log{#if pins.length}<i class="badge">{pins.length}</i>{/if}</span>
    </button>
    <button class:on={tab === "signal"} onclick={() => goTab("signal")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M3 13h3l2.5-6 4 11 3-8 2 3H21" /></svg>
      <span>Signal</span>
    </button>
    <button class:on={tab === "settings"} onclick={() => goTab("settings")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><rect x="14" y="5" width="4" height="4" /><rect x="8" y="15" width="4" height="4" /></svg>
      <span>Config</span>
    </button>
  </nav>
</main>

<!-- ── TOAST ─────────────────────────────────────────────────────────── -->
{#if toast}
  <div class="toast" data-theme={settings.theme} style={themeVars(settings.theme)} role="status">{toast}</div>
{/if}

<!-- ── ABOUT / SCAN EXPLAINER ────────────────────────────────────────── -->
{#if aboutOpen}
  <div class="sheet-backdrop" data-theme={settings.theme} style={themeVars(settings.theme)}
       onclick={() => (aboutOpen = false)} role="presentation">
    <div class="sheet" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="How scanning works">
      <div class="sheet-head">
        <span><small>Manual</small>How the scan works</span>
        <button class="icon-btn" aria-label="Close" onclick={() => (aboutOpen = false)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M6 6l12 12M18 6L6 18" /></svg>
        </button>
      </div>

      <ol class="steps">
        <li><span>01</span><p><b>Hold the phone flat</b> and turn slowly in a full circle.</p></li>
        <li><span>02</span><p><b>Watch the dial.</b> The wedge points toward the strongest direction measured so far; the big number is the strength where you're pointing.</p></li>
        <li><span>03</span><p><b>Face it.</b> Turn until the wedge sits under the top mark and it reads <i>Locked on</i> — a short buzz confirms.</p></li>
        <li><span>04</span><p><b>Mark spots.</b> Tap <b>Mark this spot</b> in a few places; the Log tab ranks them strongest-first and stars the best — handy for placing a router or picking a seat.</p></li>
      </ol>

      <div class="legend">
        <div class="leg"><span class="dotc good"></span>Strong</div>
        <div class="leg"><span class="dotc fair"></span>Fair</div>
        <div class="leg"><span class="dotc weak"></span>Weak</div>
      </div>

      <div class="callout">
        <b>What's real, what's not</b>
        <p><b>Real:</b> the compass heading (phone magnetometer) and — on Android — the <b>signal strength in dBm</b>, read live from the system. Wi-Fi is preferred; with no Wi-Fi the app falls back to <b>mobile data (cellular)</b>.</p>
        <p><b>Also measured:</b> <b>ping</b> is a real round-trip (a TCP handshake to a public server every few seconds), and <b>link</b> is the Wi-Fi rate your phone negotiated with the router — the ceiling, not a speed test. Anything that can't be measured shows <b>--</b>. Mobile signal needs the <b>Location</b> permission (an Android rule); without it only Wi-Fi works.</p>
        <p><b>No signal?</b> With no Wi-Fi and no mobile data the app says so plainly and waits — it won't invent a reading. (On desktop it shows a labelled demo so you can preview the interaction.)</p>
      </div>
    </div>
  </div>
{/if}

<style>
  :global(html, body) {
    margin: 0;
    background: #14161b;
    overscroll-behavior: none;
  }
  /* Natural document scroll — the most reliable way to reach lower content on
     a phone. (A fixed-height + inner overflow layout failed in the webview.) */
  :global(body) {
    min-height: 100dvh;
    transition: background 0.3s ease;
    -webkit-tap-highlight-color: transparent;
    -webkit-user-select: none; user-select: none;
  }

  /* ── Design language ────────────────────────────────────────────────
     A field instrument, not an app: square edges, hairline rules, corner
     registration marks, chamfered controls, condensed uppercase labels and
     monospaced readouts. No rounded cards, no soft drop shadows. */
  .app, .toast, .sheet-backdrop {
    --f-ui: "Barlow", ui-sans-serif, system-ui, sans-serif;
    --f-head: "Barlow Condensed", "Barlow", ui-sans-serif, sans-serif;
    --f-mono: "JetBrains Mono", ui-monospace, "SF Mono", monospace;
    --edge: color-mix(in srgb, var(--text) 16%, transparent);
    --mark: color-mix(in srgb, var(--text) 55%, transparent);
    --cham: 10px;
  }
  .app {
    --tabbar-h: 62px;
    --top-h: 58px;
    min-height: 100dvh;
    width: 100%;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    padding:
      env(safe-area-inset-top) env(safe-area-inset-right)
      calc(var(--tabbar-h) + env(safe-area-inset-bottom)) env(safe-area-inset-left);
    background: var(--bg);
    color: var(--text);
    font-family: var(--f-ui);
    -webkit-font-smoothing: antialiased;
    transition: background 0.3s ease, color 0.3s ease;
  }
  button { -webkit-tap-highlight-color: transparent; }
  svg { fill: none; stroke: currentColor; stroke-width: 1.7; stroke-linecap: square; stroke-linejoin: miter; }

  /* Panel: square box with bright corner registration marks. */
  .panel {
    position: relative;
    border: 1px solid var(--edge);
    background:
      linear-gradient(var(--mark), var(--mark)) top left / 9px 1px no-repeat,
      linear-gradient(var(--mark), var(--mark)) top left / 1px 9px no-repeat,
      linear-gradient(var(--mark), var(--mark)) top right / 9px 1px no-repeat,
      linear-gradient(var(--mark), var(--mark)) top right / 1px 9px no-repeat,
      linear-gradient(var(--mark), var(--mark)) bottom left / 9px 1px no-repeat,
      linear-gradient(var(--mark), var(--mark)) bottom left / 1px 9px no-repeat,
      linear-gradient(var(--mark), var(--mark)) bottom right / 9px 1px no-repeat,
      linear-gradient(var(--mark), var(--mark)) bottom right / 1px 9px no-repeat,
      var(--surface);
    background-origin: border-box;
  }
  .p-head {
    display: flex; justify-content: space-between; align-items: center; gap: 10px;
    padding: 9px 14px 8px;
    border-bottom: 1px solid var(--line);
    font-family: var(--f-head); font-size: 12.5px; font-weight: 600;
    letter-spacing: 0.14em; text-transform: uppercase; color: var(--muted);
  }

  /* ── Header ── */
  .topbar {
    position: sticky; top: 0; z-index: 20;
    height: var(--top-h); box-sizing: border-box;
    display: flex; justify-content: space-between; align-items: center;
    padding: 0 16px;
    background: var(--bg);
    border-bottom: 1px solid var(--edge);
  }
  .brand { display: inline-flex; align-items: center; gap: 10px; }
  .logo { width: 30px; height: 30px; display: block; }
  .brand-txt { display: flex; flex-direction: column; line-height: 1; gap: 4px; }
  .brand-txt b {
    font-family: var(--f-head); font-size: 21px; font-weight: 700;
    letter-spacing: 0.16em; text-transform: uppercase;
  }
  .brand-txt small {
    font-family: var(--f-mono); font-size: 9.5px; letter-spacing: 0.12em;
    text-transform: uppercase; color: var(--muted);
  }
  .src {
    display: inline-flex; align-items: stretch; height: 32px;
    font-family: var(--f-mono); font-size: 11.5px; font-weight: 500;
    letter-spacing: 0.08em; text-transform: uppercase;
    color: var(--muted); background: none; padding: 0; cursor: pointer;
    border: 1px solid var(--edge);
  }
  .src > span { display: inline-flex; align-items: center; padding: 0 9px; }
  .src .led { padding: 0; width: 7px; margin: 0 0 0 10px; }
  .src-k { border-right: 1px solid var(--edge); color: var(--muted); }
  .src-v { color: var(--text); min-width: 38px; justify-content: center; }
  .src.live { border-color: color-mix(in srgb, var(--good) 45%, transparent); }
  .src.live .src-v { color: var(--good); }
  .src.warn { border-color: color-mix(in srgb, var(--weak) 45%, transparent); }
  .src.warn .src-v { color: var(--weak); }
  .led {
    display: inline-block; width: 7px; height: 7px; align-self: center;
    background: var(--muted);
  }
  .live .led { background: var(--good); box-shadow: 0 0 6px var(--good); animation: blink 1.6s steps(2, jump-none) infinite; }
  .warn .led { background: var(--weak); }
  @keyframes blink { 0%, 60% { opacity: 1; } 61%, 100% { opacity: 0.35; } }
  .icon-btn {
    width: 40px; height: 40px;
    background: none; border: 1px solid var(--edge);
    color: var(--text); cursor: pointer;
    display: inline-flex; align-items: center; justify-content: center;
  }
  .icon-btn svg { width: 18px; height: 18px; }

  /* ── Screens ── */
  .screen {
    display: flex; flex-direction: column; gap: 12px;
    padding: 14px 16px 20px;
    animation: fade 0.16s ease-out;
  }
  .screen[hidden] { display: none; }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }

  /* Section strip: "01  DIRECTION FINDER ─────── extra" */
  .sec {
    display: flex; align-items: center; gap: 10px;
    font-family: var(--f-head); font-size: 13px; font-weight: 600;
    letter-spacing: 0.18em; text-transform: uppercase;
  }
  .sec-n {
    font-family: var(--f-mono); font-size: 11px; font-weight: 700; letter-spacing: 0;
    color: var(--bg); background: var(--accent); padding: 2px 5px;
  }
  .sec-t { color: var(--text); white-space: nowrap; }
  .sec-r { flex: 1; height: 1px; background: var(--edge); }
  .sec-x { font-family: var(--f-mono); font-size: 11px; letter-spacing: 0.06em; color: var(--muted); }

  /* ── Find ── */
  .find {
    /* Fill exactly one phone screen so the Mark button sits in thumb reach. */
    min-height: calc(100dvh - var(--top-h) - var(--tabbar-h) - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    box-sizing: border-box;
    justify-content: space-between;
    align-items: center;
  }
  .find .sec { width: 100%; }
  .telemetry {
    width: 100%; display: grid; grid-template-columns: 1.2fr 1.2fr 1fr;
    border: 1px solid var(--edge);
  }
  .telemetry div {
    display: flex; flex-direction: column; gap: 3px; padding: 7px 10px 8px;
    font-variant-numeric: tabular-nums;
  }
  .telemetry div + div { border-left: 1px solid var(--edge); }
  .telemetry small {
    font-family: var(--f-head); font-size: 10.5px; font-weight: 600;
    letter-spacing: 0.18em; color: var(--muted);
  }
  .telemetry b { font-family: var(--f-mono); font-size: 15px; font-weight: 500; }
  .telemetry i { font-style: normal; font-size: 10.5px; color: var(--muted); margin-left: 4px; }
  .telemetry .live b { color: var(--good); }

  .dial-wrap {
    position: relative;
    width: min(86vw, 380px, 44dvh);
    aspect-ratio: 1;
    container-type: inline-size;
  }
  /* Viewfinder brackets around the dial. */
  .dial-wrap::before {
    content: ""; position: absolute; inset: -6px; pointer-events: none;
    --b: color-mix(in srgb, var(--text) 30%, transparent);
    background:
      linear-gradient(var(--b), var(--b)) top left / 16px 1px no-repeat,
      linear-gradient(var(--b), var(--b)) top left / 1px 16px no-repeat,
      linear-gradient(var(--b), var(--b)) top right / 16px 1px no-repeat,
      linear-gradient(var(--b), var(--b)) top right / 1px 16px no-repeat,
      linear-gradient(var(--b), var(--b)) bottom left / 16px 1px no-repeat,
      linear-gradient(var(--b), var(--b)) bottom left / 1px 16px no-repeat,
      linear-gradient(var(--b), var(--b)) bottom right / 16px 1px no-repeat,
      linear-gradient(var(--b), var(--b)) bottom right / 1px 16px no-repeat;
  }
  .dial { display: block; width: 100%; height: 100%; }
  .dial-center {
    position: absolute; inset: 0; pointer-events: none;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .q-cap {
    font-family: var(--f-mono); font-size: max(9px, 3.2cqw); letter-spacing: 0.4em;
    color: var(--muted); padding-left: 0.4em;
  }
  .q-big {
    font-family: var(--f-mono); font-size: 19cqw; font-weight: 700; line-height: 1;
    letter-spacing: -0.04em; font-variant-numeric: tabular-nums; margin-top: 1.2cqw;
  }
  .q-big span { font-size: 0.36em; font-weight: 500; margin-left: 0.1em; opacity: 0.7; letter-spacing: 0; }
  .q-big.none { color: var(--muted); }
  .q-label {
    font-family: var(--f-head); font-size: max(12px, 4.6cqw); font-weight: 700;
    letter-spacing: 0.22em; text-transform: uppercase; margin-top: 1.6cqw; color: var(--muted);
    padding-left: 0.22em;
  }
  .segs { display: flex; gap: 0.9cqw; margin-top: 3cqw; }
  .seg { width: 2.6cqw; height: 3.4cqw; min-height: 7px; background: color-mix(in srgb, var(--text) 12%, transparent); transition: background 0.25s; }
  .segs.good .seg.on { background: var(--good); }
  .segs.fair .seg.on { background: var(--fair); }
  .segs.weak .seg.on { background: var(--weak); }
  .q-head {
    font-family: var(--f-mono); font-size: max(10px, 3.5cqw); color: var(--muted);
    margin-top: 3cqw; font-variant-numeric: tabular-nums; letter-spacing: 0.06em;
  }
  .good { color: var(--good); }
  .fair { color: var(--fair); }
  .weak { color: var(--weak); }

  .guide {
    display: flex; align-items: stretch;
    width: 100%; box-sizing: border-box; min-height: 60px;
    border: 1px solid var(--edge); background: var(--surface);
    transition: background 0.2s, border-color 0.2s;
  }
  .guide.aligned { background: color-mix(in srgb, var(--good) 12%, var(--surface)); border-color: var(--good); }
  .guide.warn { background: color-mix(in srgb, var(--weak) 9%, var(--surface)); border-color: color-mix(in srgb, var(--weak) 55%, transparent); }
  .guide-ico {
    width: 58px; flex-shrink: 0;
    display: grid; place-items: center;
    background: var(--accent); color: var(--bg);
    font-family: var(--f-mono); font-size: 24px; font-weight: 700;
  }
  .guide-ico svg { width: 28px; height: 28px; stroke-width: 2.6; transition: transform 0.3s cubic-bezier(0.22,1,0.36,1); }
  .guide.aligned .guide-ico { background: var(--good); }
  .guide.warn .guide-ico { background: var(--weak); }
  .guide-text { flex: 1; display: flex; flex-direction: column; justify-content: center; gap: 2px; min-width: 0; padding: 8px 14px; }
  .guide-text b {
    font-family: var(--f-head); font-size: 21px; font-weight: 700;
    letter-spacing: 0.08em; text-transform: uppercase; line-height: 1.05;
  }
  .guide-text small { font-size: 13px; color: var(--muted); line-height: 1.3; }
  .guide.aligned .guide-text b { color: var(--good); }
  .guide-deg {
    display: grid; place-items: center; padding: 0 14px;
    border-left: 1px solid var(--edge);
    font-family: var(--f-mono); font-size: 16px; font-weight: 500;
    font-variant-numeric: tabular-nums; color: var(--accent);
  }
  .guide.aligned .guide-deg { color: var(--good); border-color: color-mix(in srgb, var(--good) 40%, transparent); }

  .find-actions { width: 100%; display: flex; flex-direction: column; align-items: center; gap: 9px; }
  /* Chamfered (cut-corner) primary key. */
  .primary-btn {
    width: 100%; min-height: 56px;
    display: flex; align-items: center; justify-content: space-between; gap: 10px;
    font-family: var(--f-head); font-size: 18px; font-weight: 700;
    letter-spacing: 0.14em; text-transform: uppercase;
    color: var(--bg); background: var(--accent);
    border: none; padding: 0 18px; cursor: pointer;
    clip-path: polygon(var(--cham) 0, 100% 0, 100% calc(100% - var(--cham)), calc(100% - var(--cham)) 100%, 0 100%, 0 var(--cham));
    transition: filter 0.1s;
  }
  .pb-l { display: inline-flex; align-items: center; gap: 10px; }
  .pb-r {
    font-family: var(--f-mono); font-size: 12.5px; font-weight: 500; letter-spacing: 0.04em;
    padding-left: 12px; border-left: 1px solid color-mix(in srgb, var(--bg) 35%, transparent);
    align-self: stretch; display: inline-flex; align-items: center;
  }
  .primary-btn svg { width: 20px; height: 20px; stroke-width: 2.2; }
  .primary-btn:active { filter: brightness(0.88); }
  .primary-btn:disabled {
    justify-content: center; background: var(--surface-2); color: var(--muted);
    cursor: default; filter: none;
  }
  .hint { margin: 0; font-size: 12.5px; color: var(--muted); text-align: center; line-height: 1.45; }

  /* ── Log (spots) ── */
  .empty-state {
    display: flex; flex-direction: column; align-items: center; text-align: center; gap: 10px;
    padding: 44px 24px 36px;
  }
  .empty-state svg { width: 46px; height: 46px; color: var(--accent); stroke-width: 1.4; }
  .empty-state > b { font-family: var(--f-head); font-size: 20px; letter-spacing: 0.14em; text-transform: uppercase; }
  .empty-state p { margin: 0; font-size: 14px; color: var(--muted); line-height: 1.5; max-width: 290px; }
  .empty-state p b { color: var(--text); }
  .ghost-btn {
    margin-top: 10px;
    font-family: var(--f-head); font-size: 15px; font-weight: 700;
    letter-spacing: 0.14em; text-transform: uppercase;
    color: var(--accent); background: none;
    border: 1px solid var(--accent); padding: 11px 22px; cursor: pointer;
  }
  .ghost-btn:active { background: color-mix(in srgb, var(--accent) 14%, transparent); }
  .best-card { border-color: color-mix(in srgb, var(--good) 45%, transparent); }
  .best-card .p-head { color: var(--good); }
  .best-row { display: flex; align-items: center; gap: 18px; padding: 14px 16px 16px; }
  .best-q { font-family: var(--f-mono); font-size: 46px; font-weight: 700; line-height: 1; letter-spacing: -0.04em; font-variant-numeric: tabular-nums; }
  .best-q small { font-size: 0.4em; font-weight: 500; letter-spacing: 0; }
  .best-info { display: flex; flex-direction: column; gap: 4px; padding-left: 18px; border-left: 1px solid var(--edge); }
  .best-info small { font-family: var(--f-head); font-size: 11px; letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted); }
  .best-info b { font-family: var(--f-mono); font-size: 18px; font-weight: 500; }
  .link {
    font-family: var(--f-head); font-size: 12.5px; font-weight: 700; letter-spacing: 0.14em; text-transform: uppercase;
    color: var(--accent); background: none; border: none; cursor: pointer; padding: 4px 0;
  }
  .spot { display: flex; align-items: center; gap: 12px; padding: 10px 6px 10px 0; min-height: 62px; box-sizing: border-box; }
  .spot + .spot { border-top: 1px solid var(--line); }
  .spot.best { background: color-mix(in srgb, var(--good) 6%, transparent); }
  .spot-rank {
    width: 40px; align-self: stretch; flex-shrink: 0;
    display: grid; place-items: center;
    font-family: var(--f-mono); font-size: 13px; font-weight: 700; color: var(--muted);
    border-right: 1px solid var(--line); margin: -10px 0;
  }
  .spot.best .spot-rank { color: var(--bg); background: var(--good); border-color: var(--good); }
  .spot-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .spot-dir { font-family: var(--f-mono); font-size: 15px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .spot-sub { font-family: var(--f-head); font-size: 11.5px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted); }
  .spot-q { font-family: var(--f-mono); font-size: 16px; font-weight: 700; min-width: 44px; text-align: right; font-variant-numeric: tabular-nums; }
  .spot-del {
    width: 38px; height: 38px; flex-shrink: 0;
    display: grid; place-items: center; color: var(--muted);
    background: none; border: 1px solid transparent; cursor: pointer;
  }
  .spot-del svg { width: 16px; height: 16px; }
  .spot-del:active { border-color: var(--weak); color: var(--weak); }

  /* ── Signal ── */
  .sim-tag { display: inline-flex; align-items: center; gap: 7px; color: var(--muted); }
  .sim-tag.live { color: var(--good); }
  .sim-tag.warn { color: var(--weak); }
  .hero-body { padding: 14px 14px 12px; }
  .hero-val {
    font-family: var(--f-mono); font-size: 54px; font-weight: 700;
    letter-spacing: -0.04em; line-height: 1; font-variant-numeric: tabular-nums;
  }
  .hero-val small { font-size: 16px; font-weight: 500; color: var(--muted); margin-left: 8px; letter-spacing: 0; }
  .hero-sub {
    font-family: var(--f-head); font-size: 15px; font-weight: 700; letter-spacing: 0.2em; text-transform: uppercase;
    margin-top: 6px; color: var(--muted);
  }
  .hero-sub.good { color: var(--good); } .hero-sub.fair { color: var(--fair); } .hero-sub.weak { color: var(--weak); }
  .spark-box {
    position: relative; margin-top: 14px;
    border: 1px solid var(--edge);
    background: color-mix(in srgb, var(--bg) 55%, transparent);
  }
  .spark { display: block; width: 100%; height: 112px; }
  .spark .grid { stroke: var(--line); stroke-width: 1; vector-effect: non-scaling-stroke; }
  .spark .grid.v { stroke-dasharray: 2 3; }
  .spark .line { stroke: var(--accent); stroke-width: 1.8; vector-effect: non-scaling-stroke; stroke-linejoin: miter; }
  .spark-y { position: absolute; left: 5px; font-family: var(--f-mono); font-size: 9px; color: var(--muted); }
  .spark-y.top { top: 3px; } .spark-y.mid { top: calc(50% - 6px); }
  .spark-axis {
    display: flex; justify-content: space-between; margin-top: 5px;
    font-family: var(--f-mono); font-size: 10px; color: var(--muted);
  }
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); border-top: 1px solid var(--line); }
  .stats div { padding: 9px 12px 11px; display: flex; flex-direction: column; gap: 3px; }
  .stats div + div { border-left: 1px solid var(--line); }
  .stats small, .mini-l, .r-label {
    font-family: var(--f-head); font-size: 11px; font-weight: 600;
    letter-spacing: 0.18em; text-transform: uppercase; color: var(--muted);
  }
  .stats b { font-family: var(--f-mono); font-size: 19px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .stats i { font-style: normal; font-size: 11px; color: var(--muted); margin-left: 2px; }
  .rows { display: flex; flex-direction: column; gap: 14px; padding: 14px; }
  .row { display: flex; align-items: center; gap: 12px; }
  .r-label { width: 48px; flex-shrink: 0; font-size: 12px; }
  .r-val { font-family: var(--f-mono); font-size: 15px; font-weight: 500; width: 44px; text-align: right; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .mini-v small { font-size: 10.5px; font-weight: 400; color: var(--muted); margin-left: 3px; }
  .bar { flex: 1; display: block; height: 10px; background: color-mix(in srgb, var(--text) 8%, transparent); overflow: hidden; position: relative; }
  .bar.sm { flex: none; height: 4px; }
  /* Segmented meter: cut the fill into discrete ticks. */
  .bar.ticks::after {
    content: ""; position: absolute; inset: 0;
    background: repeating-linear-gradient(90deg, transparent 0 5px, var(--surface) 5px 7px);
  }
  .fill { display: block; height: 100%; background: var(--accent); transition: width 0.4s cubic-bezier(0.22,1,0.36,1); }
  .fill.good { background: var(--good); } .fill.fair { background: var(--fair); } .fill.weak { background: var(--weak); }
  .mini-row { display: grid; grid-template-columns: repeat(3, 1fr); border-top: 1px solid var(--line); }
  .mini { padding: 10px 12px 12px; display: flex; flex-direction: column; gap: 4px; }
  .mini + .mini { border-left: 1px solid var(--line); }
  .mini-v { font-family: var(--f-mono); font-size: 20px; font-weight: 500; font-variant-numeric: tabular-nums; }
  .note { margin: 0; padding: 12px 14px; border-top: 1px solid var(--line); font-size: 13px; color: var(--muted); line-height: 1.5; }
  .note b { color: var(--text); }

  /* ── Config ── */
  .set-group { display: flex; flex-direction: column; gap: 8px; margin-top: 4px; }
  .set-title {
    display: flex; align-items: center; gap: 8px;
    font-family: var(--f-head); font-size: 13px; font-weight: 700;
    letter-spacing: 0.18em; text-transform: uppercase; color: var(--text);
  }
  .set-title span {
    font-family: var(--f-mono); font-size: 10.5px; letter-spacing: 0; color: var(--accent);
    border: 1px solid var(--accent); width: 18px; height: 18px; display: grid; place-items: center;
  }
  .set-list { display: flex; flex-direction: column; border: 1px solid var(--edge); background: var(--surface); }
  .set-note { margin: 0; font-size: 12.5px; color: var(--muted); line-height: 1.5; }
  .set-note b { color: var(--text); }
  .theme-group-label {
    font-family: var(--f-mono); font-size: 10px; letter-spacing: 0.14em; text-transform: uppercase;
    color: var(--muted); margin-top: 2px;
  }
  .themes { display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px; }
  .theme-chip {
    display: flex; flex-direction: column; align-items: stretch; gap: 7px;
    font-family: var(--f-head); font-size: 13.5px; font-weight: 600; letter-spacing: 0.1em; text-transform: uppercase;
    color: var(--text); background: var(--surface); border: 1px solid var(--edge);
    padding: 6px 6px 8px; cursor: pointer; text-align: left;
  }
  .theme-chip.on { border-color: var(--accent); box-shadow: inset 0 -3px 0 var(--accent); }
  /* Only the swatch paints with the candidate theme's vars (set inline). */
  .sw-preview { position: relative; height: 30px; overflow: hidden; border: 1px solid var(--edge); }
  .sw-bg { position: absolute; inset: 0; background: var(--bg); }
  .sw-accent { position: absolute; left: 0; bottom: 0; top: 0; width: 34%; background: var(--accent); }
  .sw-label { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; padding-left: 2px; }
  .seg-pick { display: grid; grid-template-columns: repeat(3, 1fr); border: 1px solid var(--edge); }
  .seg-opt {
    min-height: 44px;
    font-family: var(--f-head); font-size: 14px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase;
    color: var(--muted); background: var(--surface); border: none; cursor: pointer;
  }
  .seg-opt + .seg-opt { border-left: 1px solid var(--edge); }
  .seg-opt.on { color: var(--bg); background: var(--accent); }
  .toggle {
    display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 62px;
    font: inherit; text-align: left; color: var(--text);
    background: none; border: none; padding: 11px 14px; cursor: pointer;
  }
  .toggle + .toggle { border-top: 1px solid var(--line); }
  .toggle:active { background: color-mix(in srgb, var(--text) 4%, transparent); }
  .toggle b { display: block; font-size: 15px; font-weight: 600; }
  .toggle small { display: block; font-size: 12.5px; color: var(--muted); margin-top: 2px; line-height: 1.35; }
  .chev { color: var(--muted); display: grid; }
  .chev svg { width: 18px; height: 18px; }
  /* Square slide switch with an ON/OFF legend. */
  .sw-track {
    width: 52px; height: 26px; flex-shrink: 0; position: relative;
    border: 1px solid var(--edge); background: var(--surface-2);
    transition: background 0.15s, border-color 0.15s;
  }
  .sw-track::after {
    content: "OFF"; position: absolute; right: 6px; top: 50%; transform: translateY(-50%);
    font-family: var(--f-mono); font-size: 8.5px; font-weight: 700; color: var(--muted);
  }
  .sw-track.on { background: var(--accent); border-color: var(--accent); }
  .sw-track.on::after { content: "ON"; right: auto; left: 7px; color: var(--bg); }
  .knob {
    position: absolute; top: 3px; left: 3px; width: 18px; height: 18px;
    background: var(--muted); transition: transform 0.15s cubic-bezier(0.22,1,0.36,1), background 0.15s;
  }
  .sw-track.on .knob { transform: translateX(26px); background: var(--bg); }
  .danger-row { display: grid; grid-template-columns: 1fr 1fr; gap: 6px; }
  .danger {
    min-height: 48px;
    font-family: var(--f-head); font-size: 14px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase;
    color: var(--weak); background: none; border: 1px solid color-mix(in srgb, var(--weak) 55%, transparent);
    cursor: pointer;
  }
  .danger:disabled { opacity: 0.4; cursor: default; }
  .danger.soft { color: var(--muted); border-color: var(--edge); }
  .ver {
    display: flex; justify-content: space-between; margin-top: 10px; padding-top: 10px;
    border-top: 1px dashed var(--edge);
    font-family: var(--f-mono); font-size: 10.5px; letter-spacing: 0.1em; text-transform: uppercase; color: var(--muted);
  }

  /* ── Tab bar ── */
  .tabbar {
    position: fixed; left: 0; right: 0; bottom: 0; z-index: 30;
    display: grid; grid-template-columns: repeat(4, 1fr);
    height: calc(var(--tabbar-h) + env(safe-area-inset-bottom));
    padding: 0 env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
    box-sizing: border-box;
    background: var(--bg);
    border-top: 1px solid var(--edge);
  }
  .tabbar button {
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 4px;
    font-family: var(--f-head); font-size: 12px; font-weight: 700; letter-spacing: 0.16em; text-transform: uppercase;
    color: var(--muted); background: none; border: none; cursor: pointer; position: relative;
  }
  .tabbar button + button { border-left: 1px solid var(--line); }
  .tabbar svg { width: 22px; height: 22px; }
  .tabbar button.on { color: var(--accent); background: color-mix(in srgb, var(--accent) 7%, transparent); }
  .tabbar button.on::before {
    content: ""; position: absolute; top: -1px; left: 0; right: 0; height: 2px; background: var(--accent);
  }
  .badge {
    font-style: normal; font-family: var(--f-mono); font-size: 9.5px; font-weight: 700; letter-spacing: 0;
    margin-left: 5px; background: var(--accent); color: var(--bg); padding: 0 4px;
  }

  /* ── Toast ── */
  .toast {
    position: fixed; left: 16px; right: 16px;
    bottom: calc(var(--tabbar-h, 62px) + 12px + env(safe-area-inset-bottom)); z-index: 60;
    background: var(--surface-2); color: var(--text);
    border: 1px solid var(--edge); border-left: 3px solid var(--accent);
    padding: 12px 14px;
    font-family: var(--f-mono); font-size: 12.5px; letter-spacing: 0.04em;
    animation: toast-in 0.16s ease-out;
  }
  @keyframes toast-in { from { opacity: 0; transform: translateY(6px); } to { opacity: 1; transform: none; } }

  /* ── Manual sheet ── */
  .sheet-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.6); display: flex; align-items: flex-end; justify-content: center; z-index: 50; animation: fade 0.18s ease; }
  .sheet {
    width: 100%; max-width: 520px; max-height: 88dvh; overflow-y: auto; box-sizing: border-box;
    background: var(--bg); color: var(--text);
    border-top: 2px solid var(--accent);
    padding: 16px 18px calc(24px + env(safe-area-inset-bottom));
    display: flex; flex-direction: column; gap: 16px;
    font-family: var(--f-ui);
    animation: rise 0.22s cubic-bezier(0.22,1,0.36,1);
  }
  @keyframes rise { from { transform: translateY(24px); opacity: 0.6; } to { transform: none; opacity: 1; } }
  .sheet-head { display: flex; justify-content: space-between; align-items: center; padding-bottom: 12px; border-bottom: 1px solid var(--edge); }
  .sheet-head > span {
    display: flex; flex-direction: column; gap: 3px;
    font-family: var(--f-head); font-size: 20px; font-weight: 700; letter-spacing: 0.12em; text-transform: uppercase;
  }
  .sheet-head small { font-family: var(--f-mono); font-size: 10px; letter-spacing: 0.16em; color: var(--accent); }
  .steps { margin: 0; padding: 0; list-style: none; display: flex; flex-direction: column; }
  .steps li { display: flex; gap: 12px; padding: 10px 0; }
  .steps li + li { border-top: 1px solid var(--line); }
  .steps li > span { font-family: var(--f-mono); font-size: 12px; font-weight: 700; color: var(--accent); padding-top: 2px; }
  .steps p { margin: 0; font-size: 14px; color: var(--muted); line-height: 1.5; }
  .steps b { color: var(--text); }
  .steps i { color: var(--accent); font-style: normal; font-weight: 600; }
  .legend { display: flex; flex-wrap: wrap; gap: 18px; }
  .leg {
    display: inline-flex; align-items: center; gap: 8px;
    font-family: var(--f-head); font-size: 12.5px; font-weight: 600; letter-spacing: 0.14em; text-transform: uppercase; color: var(--muted);
  }
  .dotc { width: 10px; height: 10px; }
  .dotc.good { background: var(--good); } .dotc.fair { background: var(--fair); } .dotc.weak { background: var(--weak); }
  .callout { border: 1px solid var(--edge); border-left: 3px solid var(--accent); background: var(--surface); padding: 12px 14px; }
  .callout > b { font-family: var(--f-head); font-size: 14px; letter-spacing: 0.14em; text-transform: uppercase; }
  .callout p { margin: 8px 0 0; font-size: 13px; color: var(--muted); line-height: 1.55; }
  .callout p b { color: var(--text); }

  /* ── Per-theme character ── */
  /* Blueprint: drafting grid behind everything, dashed panel rules. */
  .app[data-theme="blueprint"] {
    background-image:
      linear-gradient(rgba(120,200,255,.07) 1px, transparent 1px),
      linear-gradient(90deg, rgba(120,200,255,.07) 1px, transparent 1px),
      linear-gradient(rgba(120,200,255,.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(120,200,255,.03) 1px, transparent 1px);
    background-size: 64px 64px, 64px 64px, 16px 16px, 16px 16px;
  }
  .app[data-theme="blueprint"] .panel,
  .app[data-theme="blueprint"] .guide,
  .app[data-theme="blueprint"] .telemetry { border-style: dashed; }
  /* Phosphor: CRT glow on lit elements + faint scanlines. */
  .app[data-theme="phosphor"] .q-big,
  .app[data-theme="phosphor"] .hero-val,
  .app[data-theme="phosphor"] .best-q,
  .app[data-theme="phosphor"] .q-label,
  .app[data-theme="phosphor"] .telemetry b { text-shadow: 0 0 12px rgba(70,214,126,.55); }
  .app[data-theme="phosphor"] .spark .line { filter: drop-shadow(0 0 4px rgba(70,214,126,.8)); }
  .app[data-theme="phosphor"]::after {
    content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 40;
    background: repeating-linear-gradient(0deg, rgba(0,0,0,.12) 0 1px, transparent 1px 3px);
  }
  /* Paper: printed-form feel — solid ink rules, no translucency. */
  .app[data-theme="paper"] { --edge: rgba(26,28,33,.28); --mark: rgba(26,28,33,.8); }
</style>
