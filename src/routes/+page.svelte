<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";

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
      if (tab !== "find") { raf = requestAnimationFrame(draw); return; }
      const ac = accentRgb;
      const ink = inkRgb;
      ctx.clearRect(0, 0, SIZE, SIZE);

      // Instrument bezel: degree ticks every 15°, longer at the cardinals.
      // These rotate with the compass and give the dial its "device" feel.
      for (let deg = 0; deg < 360; deg += 15) {
        const major = deg % 90 === 0;
        const [x1, y1] = posFor(deg - heading, R);
        const [x2, y2] = posFor(deg - heading, R - (major ? 9 : 5));
        ctx.strokeStyle = `rgba(${ink},${major ? 0.32 : 0.14})`;
        ctx.lineWidth = major ? 1.6 : 1;
        ctx.beginPath();
        ctx.moveTo(x1, y1);
        ctx.lineTo(x2, y2);
        ctx.stroke();
      }

      // Optional concentric range rings + crosshairs (the "graticule").
      if (settings.showGraticule) {
        ctx.strokeStyle = `rgba(${ink},0.07)`;
        ctx.lineWidth = 1;
        for (let i = 1; i <= 3; i++) {
          ctx.beginPath();
          ctx.arc(c, c, ((R - 14) / 3) * i, 0, Math.PI * 2);
          ctx.stroke();
        }
        ctx.strokeStyle = `rgba(${ink},0.05)`;
        ctx.beginPath();
        ctx.moveTo(c, c - (R - 14)); ctx.lineTo(c, c + (R - 14));
        ctx.moveTo(c - (R - 14), c); ctx.lineTo(c + (R - 14), c);
        ctx.stroke();
      }

      // A soft "up = aligned" target band at the top of the bezel, so the goal
      // is obvious. Kept to the outer ring so the centre readout stays clear.
      ctx.strokeStyle = `rgba(${ac},0.10)`;
      ctx.lineWidth = 22;
      ctx.lineCap = "butt";
      ctx.beginPath();
      ctx.arc(c, c, R - 30, (-90 - 14) * Math.PI / 180, (-90 + 14) * Math.PI / 180);
      ctx.stroke();

      // Cardinal letters around the bezel (rotate with the compass).
      ctx.font = "600 10px ui-sans-serif, system-ui, sans-serif";
      ctx.textAlign = "center";
      ctx.textBaseline = "middle";
      for (const [deg, ch] of [[0, "N"], [90, "E"], [180, "S"], [270, "W"]] as [number, string][]) {
        const [lx, ly] = posFor(deg - heading, R - 18);
        ctx.fillStyle = `rgba(${ink},${ch === "N" ? 0.6 : 0.34})`;
        ctx.fillText(ch, lx, ly);
      }

      // Saved-spot markers around the dial.
      for (const p of pins) {
        const [x, y] = posFor(p.bearing - heading, R - 30);
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

      // The one pointer that matters: a lit arc + outward chevron on the bezel
      // at the strongest direction. Drawn on the ring (not through the centre)
      // so it never covers the big strength number. Hidden with no signal.
      if (signalMode !== "none") {
        const rel = bestBearing - heading;
        const a0 = ((rel - 90 - 16) * Math.PI) / 180;
        const a1 = ((rel - 90 + 16) * Math.PI) / 180;
        const rr = R - 30;
        ctx.lineCap = "round";
        ctx.strokeStyle = `rgba(${ac},0.22)`;
        ctx.lineWidth = 10;
        ctx.beginPath(); ctx.arc(c, c, rr, a0, a1); ctx.stroke();
        ctx.strokeStyle = `rgba(${ac},0.95)`;
        ctx.lineWidth = 3.5;
        ctx.beginPath(); ctx.arc(c, c, rr, a0 + 0.08, a1 - 0.08); ctx.stroke();
        // Chevron pointing outward.
        const [hx, hy] = posFor(rel, rr + 17);
        const [lx, ly] = posFor(rel - 6, rr + 7);
        const [qx, qy] = posFor(rel + 6, rr + 7);
        ctx.fillStyle = `rgba(${ac},1)`;
        ctx.beginPath(); ctx.moveTo(hx, hy); ctx.lineTo(lx, ly); ctx.lineTo(qx, qy); ctx.closePath(); ctx.fill();
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
      <span>{tab === "find" ? "Signalume" : tab === "spots" ? "Saved spots" : tab === "signal" ? "Signal" : "Settings"}</span>
    </span>
    <button class="src-chip" class:live={usingRealSignal} class:warn={noSignal}
            onclick={() => (aboutOpen = true)} aria-label="Signal source — how it works">
      <span class="dot"></span>
      {#if signalMode === "wifi"}Wi-Fi
      {:else if signalMode === "cell"}Mobile
      {:else if noSignal}No signal
      {:else}Demo{/if}
    </button>
  </header>

  <!-- ══ FIND ═══════════════════════════════════════════════════════════ -->
  <section class="screen find" hidden={tab !== "find"}>
    <div class="find-meta">
      <span class:live={usingRealSignal}>
        {#if usingRealSignal}{rssiDbm} dBm{:else if noSignal}— dBm{:else}simulated{/if}
      </span>
      <span class:live={usingRealSensor}>{usingRealSensor ? "Live compass" : "No compass"}</span>
    </div>

    <div class="dial-wrap" bind:this={dialWrap}>
      <canvas bind:this={canvas} class="dial"></canvas>
      <div class="dial-center">
        {#if noSignal}
          <div class="q-cap">SIGNAL</div>
          <div class="q-big none">—</div>
          <div class="q-label">No signal</div>
        {:else}
          <div class="q-cap">SIGNAL</div>
          <div class="q-big {qInfo.cls}">{quality}<span>%</span></div>
          <div class="q-label {qInfo.cls}">{qInfo.label}</div>
          <div class="segs" aria-hidden="true">
            {#each [1, 2, 3, 4, 5] as i}
              <span class="seg {qInfo.cls}" class:on={i <= qInfo.level}></span>
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
            <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5" /></svg>
          {:else}
            <svg viewBox="0 0 24 24" aria-hidden="true" style="transform: rotate({clamp(toBest, -150, 150)}deg)">
              <path d="M12 20V5M6 11l6-6 6 6" />
            </svg>
          {/if}
        </span>
        <span class="guide-text">
          <b>{guidance.main}</b>
          <small>{guidance.sub}</small>
        </span>
      </div>
    {/if}

    <div class="find-actions">
      <button class="primary-btn" onclick={takeReading} disabled={noSignal}>
        {#if noSignal}Waiting for signal…{:else}
          <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 21s-6.5-5.6-6.5-11a6.5 6.5 0 0113 0c0 5.4-6.5 11-6.5 11z" /><circle cx="12" cy="10" r="2.3" /></svg>
          Save this spot
        {/if}
      </button>
      {#if settings.showHint}
        <p class="hint">Turn slowly in a circle — the arrow learns where the signal is strongest.</p>
      {/if}
    </div>
  </section>

  <!-- ══ SPOTS ══════════════════════════════════════════════════════════ -->
  <section class="screen" hidden={tab !== "spots"}>
    {#if !pins.length}
      <div class="empty-state">
        <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 21s-6.5-5.6-6.5-11a6.5 6.5 0 0113 0c0 5.4-6.5 11-6.5 11z" /><circle cx="12" cy="10" r="2.3" /></svg>
        <b>No saved spots yet</b>
        <p>Walk to a few places and tap <b>Save this spot</b> on the Find tab. The strongest one gets a ★.</p>
        <button class="ghost-btn" onclick={() => goTab("find")}>Go to Find</button>
      </div>
    {:else}
      {#if bestPin}
        <div class="card best-card">
          <span class="best-cap">★ Best spot</span>
          <div class="best-row">
            <span class="best-q {qualityClass(bestPin.quality)}">{bestPin.quality}<small>%</small></span>
            <span class="best-info">
              <b>Facing {bestPin.point} · {bestPin.bearing}°</b>
              <small>{bestPin.src ? PIN_SRC_LABEL[bestPin.src] : ""}{bestPin.src && bestPin.t ? " · " : ""}{pinTime(bestPin.t)}</small>
            </span>
          </div>
        </div>
      {/if}
      <div class="card list-card">
        <div class="card-head">
          <span>{pins.length} {pins.length === 1 ? "spot" : "spots"} · strongest first</span>
          <button class="link" onclick={clearPins}>Clear all</button>
        </div>
        {#each sortedPins as p, i (p.id)}
          <div class="spot" class:best={bestPin && p.id === bestPin.id}>
            <span class="spot-rank">{i + 1}</span>
            <span class="spot-main">
              <span class="spot-dir">{p.point} · {p.bearing}°</span>
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
    <div class="card hero">
      <div class="hero-top">
        <span class="sim-tag" class:live={usingRealSignal} class:warn={noSignal}>{detailsTag}</span>
        <span class="hero-src">
          {#if signalMode === "wifi"}Wi-Fi{:else if signalMode === "cell"}Mobile data{:else if noSignal}No source{:else}Demo{/if}
        </span>
      </div>
      <div class="hero-val">
        {#if usingRealSignal}{rssiDbm}<small>dBm</small>
        {:else if noSignal}—
        {:else}{quality}<small>%</small>{/if}
      </div>
      <div class="hero-sub {noSignal ? '' : qInfo.cls}">{noSignal ? "Waiting for a signal" : qInfo.label}</div>

      <svg class="spark" viewBox="0 0 {SPARK_W} {SPARK_H}" preserveAspectRatio="none" aria-label="Signal over the last two minutes">
        {#each [25, 50, 75] as g}
          <line x1="0" x2={SPARK_W} y1={SPARK_H - (g / 100) * SPARK_H} y2={SPARK_H - (g / 100) * SPARK_H} class="grid" />
        {/each}
        {#if sparkPath}<path d={sparkPath} class="line" />{/if}
      </svg>
      <div class="spark-axis"><span>2 min ago</span><span>now</span></div>

      <div class="stats">
        <div><small>Low</small><b>{histStats ? histStats.min + "%" : "—"}</b></div>
        <div><small>Average</small><b>{histStats ? histStats.avg + "%" : "—"}</b></div>
        <div><small>High</small><b>{histStats ? histStats.max + "%" : "—"}</b></div>
      </div>
    </div>

    <div class="card det-body">
      <div class="row">
        <span class="r-label">Wi-Fi</span>
        <span class="bar"><span class="fill" style="width:{wifiPct}%"></span></span>
        <span class="r-val">{#if wifiDbm != null}{wifiDbm}<small>dBm</small>{:else}—{/if}</span>
      </div>
      <div class="row">
        <span class="r-label">Cellular</span>
        <span class="bar"><span class="fill" style="width:{cellPct}%"></span></span>
        <span class="r-val">{#if cellDbm != null}{cellDbm}<small>dBm</small>{:else}—{/if}</span>
      </div>
      <div class="mini-row">
        <div class="mini"><span class="mini-l">Ping</span><span class="mini-v">{#if pingMs != null}{pingMs}<small>ms</small>{:else}—{/if}</span></div>
        <div class="mini"><span class="mini-l">Link</span><span class="mini-v">{#if linkMbps != null}{linkMbps}<small>Mb/s</small>{:else}—{/if}</span></div>
        <div class="mini"><span class="mini-l">Heading</span><span class="mini-v">{Math.round(shownHeading)}<small>°</small></span></div>
      </div>
      {#if !locationGranted}
        <p class="note">Mobile signal needs the <b>Location</b> permission. Allow it in Android settings → Apps → Signalume.</p>
      {/if}
    </div>
  </section>

  <!-- ══ SETTINGS ═══════════════════════════════════════════════════════ -->
  <section class="screen" hidden={tab !== "settings"}>
    <div class="set-group">
      <div class="set-title">Theme</div>
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
      <div class="set-title">Compass steadiness</div>
      <div class="seg-pick">
        {#each [["responsive","Responsive"],["balanced","Balanced"],["steady","Steady"]] as [id, label]}
          <button class="seg-opt" class:on={settings.smoothing === id}
                  onclick={() => (settings.smoothing = id as SmoothId)}>{label}</button>
        {/each}
      </div>
      <p class="set-note">Pick <b>Steady</b> if the numbers jump around too much.</p>
    </div>

    <div class="set-group">
      <div class="set-title">Feedback</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (settings.buzzOnLock = !settings.buzzOnLock)}>
          <span><b>Buzz when aligned</b><small>Vibrate the instant you face the strongest signal</small></span>
          <span class="sw-track" class:on={settings.buzzOnLock}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.haptics = !settings.haptics)}>
          <span><b>Haptic feedback</b><small>Vibrate when saving a spot</small></span>
          <span class="sw-track" class:on={settings.haptics}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.keepAwake = !settings.keepAwake)}>
          <span><b>Keep screen on</b><small>Stop the display sleeping while hunting</small></span>
          <span class="sw-track" class:on={settings.keepAwake}><span class="knob"></span></span>
        </button>
      </div>
    </div>

    <div class="set-group">
      <div class="set-title">Dial</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (settings.showReadout = !settings.showReadout)}>
          <span><b>Heading readout</b><small>The “184° S” line inside the dial</small></span>
          <span class="sw-track" class:on={settings.showReadout}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.showGraticule = !settings.showGraticule)}>
          <span><b>Graticule</b><small>Faint rings and crosshairs behind the arrow</small></span>
          <span class="sw-track" class:on={settings.showGraticule}><span class="knob"></span></span>
        </button>
        <button class="toggle" onclick={() => (settings.showHint = !settings.showHint)}>
          <span><b>Helper text</b><small>The short tip under the Save button</small></span>
          <span class="sw-track" class:on={settings.showHint}><span class="knob"></span></span>
        </button>
      </div>
    </div>

    <div class="set-group">
      <div class="set-title">Data</div>
      <div class="set-list">
        <button class="toggle" onclick={() => (aboutOpen = true)}>
          <span><b>How the scan works</b><small>What's measured and what isn't</small></span>
          <span class="chev">›</span>
        </button>
        <button class="danger" onclick={clearPins} disabled={!pins.length}>Clear saved spots ({pins.length})</button>
        <button class="danger soft" onclick={() => { settings = { ...DEFAULTS }; showToast("Settings reset"); }}>
          Reset settings to defaults
        </button>
      </div>
    </div>

    <div class="ver">Signalume · v0.1</div>
  </section>

  <!-- ── TAB BAR ─────────────────────────────────────────────────────── -->
  <nav class="tabbar" aria-label="Sections">
    <button class:on={tab === "find"} onclick={() => goTab("find")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="8.5" /><path d="M15.5 8.5l-2 5-5 2 2-5z" /></svg>
      <span>Find</span>
    </button>
    <button class:on={tab === "spots"} onclick={() => goTab("spots")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 21s-6.5-5.6-6.5-11a6.5 6.5 0 0113 0c0 5.4-6.5 11-6.5 11z" /><circle cx="12" cy="10" r="2.3" /></svg>
      <span>Spots{#if pins.length}<i class="badge">{pins.length}</i>{/if}</span>
    </button>
    <button class:on={tab === "signal"} onclick={() => goTab("signal")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 19v-3M10 19v-7M15 19v-10M20 19V5" /></svg>
      <span>Signal</span>
    </button>
    <button class:on={tab === "settings"} onclick={() => goTab("settings")}>
      <svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 7h10M18 7h2M4 17h4M12 17h8" /><circle cx="16" cy="7" r="2" /><circle cx="10" cy="17" r="2" /></svg>
      <span>Settings</span>
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
      <div class="grabber"></div>
      <div class="sheet-head">
        <span>How the scan works</span>
        <button class="icon-btn" aria-label="Close" onclick={() => (aboutOpen = false)}>✕</button>
      </div>

      <ol class="steps">
        <li><b>Hold the phone flat</b> and turn slowly in a full circle.</li>
        <li><b>Watch the dial.</b> The arrow points toward the strongest direction measured so far; the big number shows the strength where you're pointing.</li>
        <li><b>Face it.</b> Turn until the arrow is at the top and it reads <i>You're facing it</i> — a short buzz confirms the lock.</li>
        <li><b>Save spots.</b> Tap <b>Save this spot</b> in a few places; the Spots tab ranks them strongest-first and stars the best — handy for placing a router or picking a seat.</li>
      </ol>

      <div class="legend">
        <div class="leg"><span class="dotc good"></span>Strong / Excellent</div>
        <div class="leg"><span class="dotc fair"></span>Fair</div>
        <div class="leg"><span class="dotc weak"></span>Weak</div>
      </div>

      <div class="callout">
        <b>What's real, what's not.</b>
        <p><b>Real:</b> the compass heading (phone magnetometer) and — on Android — the <b>signal strength in dBm</b>, read live from the system. Wi-Fi is preferred; with no Wi-Fi the app falls back to <b>mobile data (cellular)</b>.</p>
        <p><b>Also measured:</b> <b>ping</b> is a real round-trip (a TCP handshake to a public server every few seconds), and <b>link</b> is the Wi-Fi rate your phone negotiated with the router — the ceiling, not a speed test. Anything that can't be measured shows <b>—</b>. Mobile signal needs the <b>Location</b> permission (an Android rule); without it only Wi-Fi works.</p>
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

  .app {
    --tabbar-h: 64px;
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
    font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
    -webkit-font-smoothing: antialiased;
    transition: background 0.3s ease, color 0.3s ease;
  }
  button { -webkit-tap-highlight-color: transparent; }
  svg { fill: none; stroke: currentColor; stroke-width: 1.8; stroke-linecap: round; stroke-linejoin: round; }

  .card {
    background: var(--surface);
    border: 1px solid var(--line);
    border-radius: 20px;
  }

  /* ── Header ── */
  .topbar {
    position: sticky; top: 0; z-index: 20;
    display: flex; justify-content: space-between; align-items: center;
    padding: 12px 18px 10px;
    background: color-mix(in srgb, var(--bg) 88%, transparent);
    backdrop-filter: blur(10px); -webkit-backdrop-filter: blur(10px);
  }
  .brand { font-size: 19px; font-weight: 700; letter-spacing: 0.2px; display: inline-flex; align-items: center; gap: 9px; }
  .logo { width: 26px; height: 26px; border-radius: 7px; }
  .src-chip {
    display: inline-flex; align-items: center; gap: 7px;
    font: inherit; font-size: 13px; font-weight: 600; color: var(--muted);
    background: var(--surface); border: 1px solid var(--line);
    padding: 8px 13px; border-radius: 999px; cursor: pointer;
  }
  .src-chip.live { color: var(--good); border-color: color-mix(in srgb, var(--good) 30%, var(--line)); }
  .src-chip.warn { color: var(--weak); border-color: color-mix(in srgb, var(--weak) 30%, var(--line)); }
  .dot { width: 7px; height: 7px; border-radius: 50%; background: currentColor; animation: pulse 1.8s ease-in-out infinite; }
  .src-chip.warn .dot { animation: none; }
  @keyframes pulse { 0%,100% { opacity: 1; } 50% { opacity: 0.35; } }
  .icon-btn {
    width: 40px; height: 40px; border-radius: 12px;
    background: var(--surface); border: 1px solid var(--line);
    color: var(--text); font-size: 16px; cursor: pointer; line-height: 1;
    display: inline-flex; align-items: center; justify-content: center;
  }

  /* ── Screens ── */
  .screen {
    display: flex; flex-direction: column; gap: 14px;
    padding: 6px 16px 20px;
    animation: fade 0.18s ease;
  }
  .screen[hidden] { display: none; }
  @keyframes fade { from { opacity: 0; transform: translateY(4px); } to { opacity: 1; transform: none; } }

  /* ── Find ── */
  .find {
    /* Fill exactly one phone screen so the Save button sits in thumb reach. */
    min-height: calc(100dvh - 58px - var(--tabbar-h) - env(safe-area-inset-top) - env(safe-area-inset-bottom));
    box-sizing: border-box;
    justify-content: space-between;
    align-items: center;
  }
  .find-meta {
    width: 100%; display: flex; justify-content: space-between;
    font-size: 12px; color: var(--muted); font-variant-numeric: tabular-nums; letter-spacing: 0.3px;
  }
  .find-meta .live { color: var(--good); }
  .dial-wrap {
    position: relative;
    width: min(88vw, 380px, 48dvh);
    aspect-ratio: 1;
    container-type: inline-size;
  }
  .dial { display: block; width: 100%; height: 100%; }
  .dial-center {
    position: absolute; inset: 0; pointer-events: none;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .q-cap { font-size: max(10px, 3.6cqw); letter-spacing: 0.35em; color: var(--muted); }
  .q-big {
    font-size: 21cqw; font-weight: 800; line-height: 1; letter-spacing: -0.03em;
    font-variant-numeric: tabular-nums; margin-top: 1cqw;
  }
  .q-big span { font-size: 0.4em; font-weight: 600; margin-left: 0.04em; opacity: 0.8; }
  .q-big.none { color: var(--muted); }
  .q-label { font-size: max(13px, 5.2cqw); font-weight: 600; margin-top: 1.5cqw; color: var(--muted); }
  .segs { display: flex; gap: 1.6cqw; margin-top: 3.5cqw; }
  .seg { width: 5.5cqw; height: 1.8cqw; min-height: 5px; border-radius: 3px; background: color-mix(in srgb, var(--text) 12%, transparent); transition: background 0.35s ease; }
  .seg.on.good { background: var(--good); }
  .seg.on.fair { background: var(--fair); }
  .seg.on.weak { background: var(--weak); }
  .q-head { font-size: max(11px, 3.9cqw); color: var(--muted); margin-top: 3.5cqw; font-variant-numeric: tabular-nums; }
  .good { color: var(--good); }
  .fair { color: var(--fair); }
  .weak { color: var(--weak); }

  .guide {
    display: flex; align-items: center; gap: 14px;
    width: 100%; box-sizing: border-box;
    background: var(--surface); border: 1px solid var(--line);
    padding: 14px 16px; border-radius: 20px;
    transition: background 0.25s ease, border-color 0.25s ease;
  }
  .guide.aligned { background: color-mix(in srgb, var(--good) 14%, var(--surface)); border-color: color-mix(in srgb, var(--good) 40%, transparent); }
  .guide.warn { background: color-mix(in srgb, var(--weak) 10%, var(--surface)); border-color: color-mix(in srgb, var(--weak) 30%, transparent); }
  .guide-ico {
    width: 48px; height: 48px; flex-shrink: 0; border-radius: 15px;
    display: grid; place-items: center;
    background: color-mix(in srgb, var(--accent) 16%, transparent); color: var(--accent);
    font-size: 22px; font-weight: 800;
  }
  .guide-ico svg { width: 28px; height: 28px; stroke-width: 2.4; transition: transform 0.35s cubic-bezier(0.22,1,0.36,1); }
  .guide.aligned .guide-ico { background: color-mix(in srgb, var(--good) 22%, transparent); color: var(--good); }
  .guide.warn .guide-ico { background: color-mix(in srgb, var(--weak) 18%, transparent); color: var(--weak); }
  .guide-text { display: flex; flex-direction: column; gap: 3px; min-width: 0; }
  .guide-text b { font-size: 18px; font-weight: 700; }
  .guide-text small { font-size: 13px; color: var(--muted); line-height: 1.35; }
  .guide.aligned .guide-text b { color: var(--good); }

  .find-actions { width: 100%; display: flex; flex-direction: column; align-items: center; gap: 10px; }
  .primary-btn {
    width: 100%; min-height: 56px;
    display: inline-flex; align-items: center; justify-content: center; gap: 9px;
    font: inherit; font-size: 16px; font-weight: 700;
    color: var(--bg); background: var(--accent);
    border: none; border-radius: 18px; padding: 0 18px; cursor: pointer;
    box-shadow: 0 8px 24px color-mix(in srgb, var(--accent) 28%, transparent);
    transition: transform 0.1s ease, filter 0.1s ease;
  }
  .primary-btn svg { width: 21px; height: 21px; stroke-width: 2.2; }
  .primary-btn:active { transform: scale(0.98); filter: brightness(0.92); }
  .primary-btn:disabled { background: var(--surface-2); color: var(--muted); box-shadow: none; cursor: default; transform: none; filter: none; }
  .hint { margin: 0; font-size: 12px; color: var(--muted); text-align: center; line-height: 1.5; }

  /* ── Spots ── */
  .empty-state {
    display: flex; flex-direction: column; align-items: center; text-align: center; gap: 10px;
    padding: 56px 24px;
  }
  .empty-state svg { width: 52px; height: 52px; color: var(--accent); stroke-width: 1.4; opacity: 0.85; }
  .empty-state > b { font-size: 18px; }
  .empty-state p { margin: 0; font-size: 14px; color: var(--muted); line-height: 1.55; max-width: 290px; }
  .empty-state p b { color: var(--text); }
  .ghost-btn {
    margin-top: 8px; font: inherit; font-size: 15px; font-weight: 600; color: var(--accent);
    background: color-mix(in srgb, var(--accent) 12%, transparent); border: none;
    padding: 12px 22px; border-radius: 14px; cursor: pointer;
  }
  .best-card {
    padding: 16px 18px;
    background: color-mix(in srgb, var(--good) 10%, var(--surface));
    border-color: color-mix(in srgb, var(--good) 30%, var(--line));
  }
  .best-cap { font-size: 12px; font-weight: 700; letter-spacing: 0.6px; text-transform: uppercase; color: var(--good); }
  .best-row { display: flex; align-items: center; gap: 16px; margin-top: 8px; }
  .best-q { font-size: 44px; font-weight: 800; line-height: 1; font-variant-numeric: tabular-nums; }
  .best-q small { font-size: 0.45em; font-weight: 600; }
  .best-info { display: flex; flex-direction: column; gap: 3px; }
  .best-info b { font-size: 16px; }
  .best-info small { font-size: 13px; color: var(--muted); }
  .list-card { overflow: hidden; }
  .card-head { display: flex; justify-content: space-between; align-items: center; padding: 14px 16px 10px; font-size: 13px; font-weight: 600; color: var(--muted); }
  .link { font: inherit; font-size: 13px; font-weight: 600; color: var(--accent); background: none; border: none; cursor: pointer; padding: 6px 0; }
  .spot { display: flex; align-items: center; gap: 12px; padding: 12px 8px 12px 16px; min-height: 64px; box-sizing: border-box; }
  .spot + .spot, .card-head + .spot { border-top: 1px solid var(--line); }
  .spot.best { background: color-mix(in srgb, var(--good) 7%, transparent); }
  .spot-rank {
    width: 28px; height: 28px; border-radius: 9px; flex-shrink: 0;
    display: grid; place-items: center; font-size: 13px; font-weight: 700;
    background: var(--surface-2); color: var(--muted);
  }
  .spot.best .spot-rank { background: color-mix(in srgb, var(--good) 22%, transparent); color: var(--good); }
  .spot-main { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 4px; }
  .spot-dir { font-size: 15px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .spot-sub { font-size: 12px; color: var(--muted); }
  .spot-q { font-size: 16px; font-weight: 700; min-width: 42px; text-align: right; font-variant-numeric: tabular-nums; }
  .spot-del {
    width: 40px; height: 40px; flex-shrink: 0; border-radius: 12px;
    display: grid; place-items: center; color: var(--muted);
    background: none; border: none; cursor: pointer;
  }
  .spot-del svg { width: 18px; height: 18px; }
  .spot-del:active { background: var(--surface-2); color: var(--weak); }

  /* ── Signal ── */
  .hero { padding: 18px 18px 16px; }
  .hero-top { display: flex; justify-content: space-between; align-items: center; }
  .hero-src { font-size: 13px; color: var(--muted); }
  .sim-tag { font-size: 11px; font-weight: 700; letter-spacing: 0.6px; text-transform: uppercase; color: var(--muted); background: color-mix(in srgb, var(--text) 7%, transparent); padding: 4px 9px; border-radius: 8px; }
  .sim-tag.live { color: var(--good); background: color-mix(in srgb, var(--good) 15%, transparent); }
  .sim-tag.warn { color: var(--weak); background: color-mix(in srgb, var(--weak) 15%, transparent); }
  .hero-val { font-size: 56px; font-weight: 800; letter-spacing: -0.03em; margin-top: 10px; line-height: 1; font-variant-numeric: tabular-nums; }
  .hero-val small { font-size: 18px; font-weight: 600; color: var(--muted); margin-left: 6px; letter-spacing: 0; }
  .hero-sub { font-size: 15px; font-weight: 600; margin-top: 6px; color: var(--muted); }
  .hero-sub.good { color: var(--good); } .hero-sub.fair { color: var(--fair); } .hero-sub.weak { color: var(--weak); }
  .spark { display: block; width: 100%; height: 110px; margin-top: 16px; }
  .spark .grid { stroke: var(--line); stroke-width: 1; vector-effect: non-scaling-stroke; }
  .spark .line { stroke: var(--accent); stroke-width: 2.2; vector-effect: non-scaling-stroke; }
  .spark-axis { display: flex; justify-content: space-between; font-size: 11px; color: var(--muted); margin-top: 4px; }
  .stats { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; margin-top: 14px; }
  .stats div { background: var(--surface-2); border-radius: 14px; padding: 10px; display: flex; flex-direction: column; align-items: center; gap: 3px; }
  .stats small { font-size: 11px; color: var(--muted); }
  .stats b { font-size: 17px; font-variant-numeric: tabular-nums; }
  .det-body { padding: 16px; display: flex; flex-direction: column; gap: 16px; }
  .row { display: flex; align-items: center; gap: 12px; }
  .r-label { font-size: 14px; color: var(--muted); width: 66px; flex-shrink: 0; }
  .r-val { font-size: 14px; font-weight: 600; color: var(--text); width: 74px; text-align: right; flex-shrink: 0; font-variant-numeric: tabular-nums; }
  .r-val small, .mini-v small { font-size: 11px; font-weight: 500; color: var(--muted); margin-left: 2px; }
  .bar { flex: 1; display: block; height: 8px; border-radius: 999px; background: color-mix(in srgb, var(--text) 8%, transparent); overflow: hidden; }
  .bar.sm { flex: none; height: 5px; }
  .fill { display: block; height: 100%; border-radius: 999px; background: var(--accent); transition: width 0.5s cubic-bezier(0.22,1,0.36,1); }
  .fill.good { background: var(--good); } .fill.fair { background: var(--fair); } .fill.weak { background: var(--weak); }
  .mini-row { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .mini { background: var(--surface-2); border-radius: 14px; padding: 12px 6px; display: flex; flex-direction: column; gap: 4px; align-items: center; }
  .mini-l { font-size: 11px; color: var(--muted); }
  .mini-v { font-size: 19px; font-weight: 700; font-variant-numeric: tabular-nums; }
  .note { margin: 0; font-size: 13px; color: var(--muted); line-height: 1.5; }
  .note b { color: var(--text); }

  /* ── Settings ── */
  .set-group { display: flex; flex-direction: column; gap: 10px; }
  .set-title { font-size: 12px; font-weight: 700; letter-spacing: 0.8px; text-transform: uppercase; color: var(--muted); padding: 6px 4px 0; }
  .set-list { display: flex; flex-direction: column; gap: 8px; }
  .set-note { margin: 0; padding: 0 4px; font-size: 12.5px; color: var(--muted); line-height: 1.5; }
  .set-note b { color: var(--text); }
  .theme-group-label { font-size: 11px; letter-spacing: 1px; text-transform: uppercase; color: var(--muted); opacity: 0.75; padding: 0 4px; }
  .themes { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
  .theme-chip {
    display: flex; align-items: center; gap: 10px; min-height: 52px;
    font: inherit; font-size: 14px; font-weight: 600; color: var(--text);
    background: var(--surface); border: 1px solid var(--line);
    padding: 8px 10px; border-radius: 14px; cursor: pointer; text-align: left;
  }
  .theme-chip.on { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
  /* Only the swatch paints with the candidate theme's vars (set inline). */
  .sw-preview { position: relative; width: 32px; height: 32px; border-radius: 9px; overflow: hidden; flex-shrink: 0; border: 1px solid var(--line); }
  .sw-bg { position: absolute; inset: 0; background: var(--bg); }
  .sw-accent { position: absolute; right: 4px; bottom: 4px; top: 4px; width: 9px; border-radius: 5px; background: var(--accent); }
  .sw-label { color: var(--text); }
  .seg-pick { display: flex; gap: 4px; background: var(--surface); border: 1px solid var(--line); border-radius: 14px; padding: 4px; }
  .seg-opt { flex: 1; min-height: 44px; font: inherit; font-size: 14px; font-weight: 600; color: var(--muted); background: none; border: none; border-radius: 11px; cursor: pointer; }
  .seg-opt.on { color: var(--bg); background: var(--accent); }
  .toggle {
    display: flex; align-items: center; justify-content: space-between; gap: 12px; min-height: 64px;
    font: inherit; text-align: left; color: var(--text);
    background: var(--surface); border: 1px solid var(--line);
    padding: 12px 16px; border-radius: 16px; cursor: pointer;
  }
  .toggle b { display: block; font-size: 15px; font-weight: 600; }
  .toggle small { display: block; font-size: 12.5px; color: var(--muted); margin-top: 3px; line-height: 1.35; }
  .chev { font-size: 24px; color: var(--muted); }
  .sw-track { width: 50px; height: 30px; border-radius: 999px; background: var(--surface-2); border: 1px solid var(--line); position: relative; flex-shrink: 0; transition: background 0.2s; }
  .sw-track.on { background: var(--accent); border-color: var(--accent); }
  .knob { position: absolute; top: 3px; left: 3px; width: 22px; height: 22px; border-radius: 50%; background: var(--muted); transition: transform 0.2s cubic-bezier(0.22,1,0.36,1), background 0.2s; }
  .sw-track.on .knob { transform: translateX(20px); background: var(--bg); }
  .danger {
    min-height: 52px; font: inherit; font-size: 15px; font-weight: 600; color: var(--weak);
    background: var(--surface); border: 1px solid color-mix(in srgb, var(--weak) 30%, var(--line));
    border-radius: 16px; cursor: pointer;
  }
  .danger:disabled { opacity: 0.45; cursor: default; }
  .danger.soft { color: var(--muted); border-color: var(--line); }
  .ver { text-align: center; font-size: 12px; color: var(--muted); opacity: 0.7; padding-top: 8px; }

  /* ── Tab bar ── */
  .tabbar {
    position: fixed; left: 0; right: 0; bottom: 0; z-index: 30;
    display: grid; grid-template-columns: repeat(4, 1fr);
    height: var(--tabbar-h);
    padding: 0 env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left);
    background: color-mix(in srgb, var(--surface) 94%, transparent);
    backdrop-filter: blur(12px); -webkit-backdrop-filter: blur(12px);
    border-top: 1px solid var(--line);
  }
  .tabbar button {
    display: flex; flex-direction: column; align-items: center; justify-content: center; gap: 3px;
    font: inherit; font-size: 11.5px; font-weight: 600; color: var(--muted);
    background: none; border: none; cursor: pointer; position: relative;
  }
  .tabbar svg { width: 25px; height: 25px; padding: 3px 16px; border-radius: 999px; transition: background 0.2s ease; }
  .tabbar button.on { color: var(--accent); }
  .tabbar button.on svg { background: color-mix(in srgb, var(--accent) 16%, transparent); }
  .badge {
    font-style: normal; font-size: 10px; font-weight: 700; margin-left: 4px;
    background: var(--accent); color: var(--bg); border-radius: 999px; padding: 0 5px;
  }

  /* ── Toast ── */
  .toast {
    position: fixed; left: 50%; transform: translateX(-50%);
    bottom: calc(var(--tabbar-h, 64px) + 14px + env(safe-area-inset-bottom)); z-index: 60;
    background: var(--surface-2); color: var(--text);
    border: 1px solid var(--line); white-space: nowrap;
    padding: 12px 18px; border-radius: 999px; font-size: 14px; font-weight: 500;
    box-shadow: 0 8px 24px rgba(0,0,0,0.3);
    animation: toast-in 0.2s ease;
  }
  @keyframes toast-in { from { opacity: 0; transform: translate(-50%, 6px); } to { opacity: 1; transform: translate(-50%, 0); } }

  /* ── About sheet ── */
  .sheet-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.55); display: flex; align-items: flex-end; justify-content: center; z-index: 50; animation: fade 0.2s ease; }
  .sheet {
    width: 100%; max-width: 520px; max-height: 88dvh; overflow-y: auto; box-sizing: border-box;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--line); border-bottom: none;
    border-radius: 24px 24px 0 0;
    padding: 10px 20px calc(24px + env(safe-area-inset-bottom));
    display: flex; flex-direction: column; gap: 16px;
    font-family: ui-sans-serif, system-ui, sans-serif;
    animation: rise 0.25s cubic-bezier(0.22,1,0.36,1);
  }
  @keyframes rise { from { transform: translateY(30px); opacity: 0.6; } to { transform: translateY(0); opacity: 1; } }
  .grabber { width: 40px; height: 4px; border-radius: 2px; background: var(--muted); opacity: 0.4; align-self: center; }
  .sheet-head { display: flex; justify-content: space-between; align-items: center; font-size: 18px; font-weight: 700; }
  .steps { margin: 0; padding-left: 20px; display: flex; flex-direction: column; gap: 10px; }
  .steps li { font-size: 14px; color: var(--muted); line-height: 1.5; }
  .steps b { color: var(--text); }
  .steps i { color: var(--accent); font-style: normal; }
  .legend { display: flex; flex-wrap: wrap; gap: 16px; padding: 2px; }
  .leg { display: inline-flex; align-items: center; gap: 7px; font-size: 13px; color: var(--muted); }
  .dotc { width: 10px; height: 10px; border-radius: 50%; }
  .dotc.good { background: var(--good); } .dotc.fair { background: var(--fair); } .dotc.weak { background: var(--weak); }
  .callout { background: var(--surface); border: 1px solid var(--line); border-radius: 16px; padding: 14px 16px; }
  .callout > b { font-size: 14px; }
  .callout p { margin: 8px 0 0; font-size: 13px; color: var(--muted); line-height: 1.55; }
  .callout p b { color: var(--text); }

  /* ── Per-theme character ── */
  /* Blueprint: faint drafting grid behind everything. */
  .app[data-theme="blueprint"] {
    background-image:
      linear-gradient(rgba(120,200,255,.06) 1px, transparent 1px),
      linear-gradient(90deg, rgba(120,200,255,.06) 1px, transparent 1px),
      linear-gradient(rgba(120,200,255,.03) 1px, transparent 1px),
      linear-gradient(90deg, rgba(120,200,255,.03) 1px, transparent 1px);
    background-size: 64px 64px, 64px 64px, 16px 16px, 16px 16px;
  }
  .app[data-theme="blueprint"] .card,
  .app[data-theme="blueprint"] .guide { border-style: dashed; }
  /* Phosphor: CRT glow on lit elements + faint scanlines. */
  .app[data-theme="phosphor"] .q-big,
  .app[data-theme="phosphor"] .hero-val,
  .app[data-theme="phosphor"] .best-q,
  .app[data-theme="phosphor"] .q-label { text-shadow: 0 0 14px rgba(70,214,126,.55); }
  .app[data-theme="phosphor"] .spark .line { filter: drop-shadow(0 0 4px rgba(70,214,126,.8)); }
  .app[data-theme="phosphor"]::after {
    content: ""; position: fixed; inset: 0; pointer-events: none; z-index: 40;
    background: repeating-linear-gradient(0deg, rgba(0,0,0,.10) 0 1px, transparent 1px 3px);
  }
</style>
