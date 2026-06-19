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
  //  Honesty note: this is a prototype. The signal field is *simulated* (there
  //  is no real Wi-Fi scan here), but the interaction is real — it responds to
  //  the phone's actual compass. Nothing in the UI pretends to scan hardware.
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
    showSpots: boolean;     // the "Saved spots" card
    showDetails: boolean;   // the technical readouts card
    showHint: boolean;      // the helper paragraph under the finder
    showGraticule: boolean; // faint rings + crosshairs on the dial
    showReadout: boolean;   // the "%· 184° N" line in the dial centre
  };
  const DEFAULTS: Settings = {
    theme: "beacon", smoothing: "balanced",
    haptics: true, buzzOnLock: true, keepAwake: false,
    showSpots: true, showDetails: false, showHint: true,
    showGraticule: true, showReadout: true,
  };
  let settings = $state<Settings>({ ...DEFAULTS });
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);
  let openSection = $state<string | null>("appearance"); // accordion: one open at a time
  let ready = $state(false); // gate persistence until stored values are loaded
  const toggleSection = (id: string) => (openSection = openSection === id ? null : id);

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

  // Signal source state, fed by the native `wifi_reading` command on Android.
  //   "demo" — desktop/browser preview only (simulated field, clearly labelled)
  //   "real" — connected to Wi-Fi, showing live RSSI
  //   "none" — on a device but no Wi-Fi signal: show an honest empty state,
  //            never a fake signal.
  let signalMode = $state<"demo" | "real" | "none">("demo");
  let usingRealSignal = $derived(signalMode === "real");
  let noSignal = $derived(signalMode === "none");
  let rssiDbm = $state(0);
  // Map a connected-AP RSSI (dBm) onto the 0..100 quality scale used everywhere
  // else: −90 dBm → unusable, −40 dBm → excellent.
  const rssiToQuality = (dbm: number) => clamp(Math.round((dbm + 90) * 2), 2, 99);
  // Best RSSI measured per 15° heading bucket — this is how the arrow learns
  // which real direction is strongest as you turn.
  let realSamples = new Map<number, number>();

  // Raw quality 0..100. Real mode reads the live RSSI; sim mode peaks when the
  // phone faces bestBearing and dips facing away.
  let qualityRaw = $derived.by(() => {
    if (signalMode === "real") return rssiToQuality(rssiDbm);
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

  // Friendly technical readouts. Wi-Fi dBm is the real RSSI when connected;
  // the rest are estimates derived from the quality value so they agree with
  // the meter. Shown only on request.
  let wifiDbm = $derived(usingRealSignal ? rssiDbm : Math.round(-92 + quality * 0.5));
  let cellDbm = $derived(Math.round(-100 + quality * 0.4));  // -100 .. -60
  let cellPct = $derived(clamp(quality - 14, 0, 100));
  let pingMs = $derived(Math.round(58 - quality * 0.46));    // ~13 .. 56
  let speedMb = $derived(Math.round(8 + quality * 0.85));    // ~10 .. 92

  // ── Saved spots ("readings") ──────────────────────────────────────────
  type Pin = { id: number; bearing: number; quality: number; point: string };
  let pins = $state<Pin[]>([]);
  let pinId = 0;

  function takeReading() {
    if (noSignal) return; // nothing meaningful to pin
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

  onMount(() => {
    // Load saved settings before enabling persistence.
    try {
      const raw = localStorage.getItem("signalume.settings");
      if (raw) settings = { ...DEFAULTS, ...JSON.parse(raw) };
    } catch {}
    if (!THEMES[settings.theme]) settings.theme = "beacon"; // migrate old ids
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

      // A soft "up = aligned" target wedge at the top, so the goal is obvious.
      ctx.fillStyle = `rgba(${ac},0.06)`;
      ctx.beginPath();
      ctx.moveTo(c, c);
      ctx.arc(c, c, R - 12, (-90 - 14) * Math.PI / 180, (-90 + 14) * Math.PI / 180);
      ctx.closePath();
      ctx.fill();

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

      // The one arrow that matters: points to the strongest direction. Hidden
      // when there's no signal source — nothing to point at.
      if (signalMode !== "none") {
        const [tx, ty] = posFor(bestBearing - heading, R - 30);
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
      }

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

    // The strongest direction drifts slowly so the app stays alive (demo only).
    const driftTimer = setInterval(() => {
      if (signalMode !== "demo") return;
      bestBearing = (bestBearing + (Math.random() - 0.5) * 6 + 360) % 360;
    }, 7000);

    // Poll the native Wi-Fi RSSI (Android only). Connected → real data; as the
    // phone turns we keep the strongest RSSI seen per heading bucket and point
    // the arrow at whichever direction measured best. On a device with no
    // connection we show an honest "no signal" state — never a fake signal.
    // The simulated field is reserved for desktop/browser preview.
    let wifiPoll: any = 0;
    const hasNative = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;
    signalMode = hasNative ? "none" : "demo"; // assume no signal until first read
    const pollWifi = async () => {
      try {
        const r = await invoke<{ rssi: number; connected: boolean }>("wifi_reading");
        if (r.connected) {
          if (signalMode !== "real") realSamples.clear(); // fresh start on (re)connect
          rssiDbm = r.rssi;
          signalMode = "real";
          const bucket = (Math.round(heading / 15) * 15 + 360) % 360;
          const prev = realSamples.get(bucket);
          // Track the *peak* per bucket — the best you could get facing that way.
          realSamples.set(bucket, prev == null ? r.rssi : Math.max(prev - 1, r.rssi));
          let bb = bucket, bv = -999;
          for (const [k, v] of realSamples) if (v > bv) { bv = v; bb = k; }
          bestBearing = bb;
        } else {
          signalMode = "none"; // associated to nothing → honest empty state
        }
      } catch (e) {
        // Desktop Tauri (command returns "only available on Android") → preview.
        if (String(e).includes("only available on Android")) {
          signalMode = "demo";
          if (wifiPoll) { clearInterval(wifiPoll); wifiPoll = 0; }
        } else {
          signalMode = "none"; // a real read error → don't fake a signal
        }
      }
    };
    if (hasNative) { pollWifi(); wifiPoll = setInterval(pollWifi, 700); }

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
      if (wifiPoll) clearInterval(wifiPoll);
      clearTimeout(toastTimer);
    };
  });
</script>

<main class="app" data-theme={settings.theme} style={themeVars(settings.theme)}>
  <!-- ── HEADER ──────────────────────────────────────────────────────── -->
  <header class="topbar">
    <span class="brand"><span class="mark">◎</span> Signalume</span>
    <div class="top-actions">
      <button class="status" class:live={usingRealSensor} onclick={() => (aboutOpen = true)}
              aria-label="What does the status mean?">
        <span class="dot"></span>{usingRealSensor ? "Live compass" : "Demo mode"}
      </button>
      <button class="icon-btn" aria-label="Settings" onclick={() => (settingsOpen = true)}>⚙</button>
    </div>
  </header>

  <!-- ── FINDER ──────────────────────────────────────────────────────── -->
  <section class="card finder">
    <div class="finder-head">
      <span class="scan-state" class:live={usingRealSignal} class:warn={noSignal}>
        <span class="scan-dot"></span>
        {#if usingRealSignal}Live Wi-Fi · {rssiDbm} dBm
        {:else if noSignal}No Wi-Fi — connect to scan
        {:else}Demo signal — turn slowly{/if}
      </span>
      <button class="link tiny" onclick={() => (aboutOpen = true)}>How it works</button>
    </div>

    <div class="dial-wrap">
      <canvas bind:this={canvas} class="dial" style="width:236px;height:236px;"></canvas>
      <div class="dial-center">
        {#if noSignal}
          <div class="q-cap">SIGNAL</div>
          <div class="q-label none">No signal</div>
          <div class="q-sub">Connect to Wi-Fi to scan</div>
          {#if settings.showReadout}
            <div class="q-head">{Math.round(shownHeading)}° {cardinal}</div>
          {/if}
        {:else}
          <div class="q-cap">SIGNAL</div>
          <div class="q-label {qInfo.cls}">{qInfo.label}</div>
          <div class="segs" aria-hidden="true">
            {#each [1, 2, 3, 4, 5] as i}
              <span class="seg {qInfo.cls}" class:on={i <= qInfo.level}></span>
            {/each}
          </div>
          {#if settings.showReadout}
            <div class="q-head">{quality}% · {Math.round(shownHeading)}° {cardinal}</div>
          {/if}
        {/if}
      </div>
    </div>

    <!-- The single, plain-language instruction. -->
    {#if noSignal}
      <div class="guide warn">
        <span class="guide-arrow">⚠</span>
        <span class="guide-text">
          <b>No signal source</b>
          <small>Connect to a Wi-Fi network, then turn slowly to scan</small>
        </span>
      </div>
    {:else}
      <div class="guide" class:aligned={guidance.aligned}>
        <span class="guide-arrow">{guidance.arrow}</span>
        <span class="guide-text">
          <b>{guidance.main}</b>
          <small>{guidance.sub}</small>
        </span>
      </div>
    {/if}

    <button class="primary-btn" onclick={takeReading} disabled={noSignal}>
      {noSignal ? "Waiting for signal…" : "Save this spot"}
    </button>
    {#if settings.showHint}
      <p class="hint">
        Turn slowly until the arrow points up and it says you're facing it. Then
        tap <b>Save this spot</b> to compare places and find the best one.
      </p>
    {/if}
  </section>

  <!-- ── SAVED SPOTS ─────────────────────────────────────────────────── -->
  {#if settings.showSpots}
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
  {/if}

  <!-- ── TECHNICAL DETAILS (optional) ────────────────────────────────── -->
  {#if settings.showDetails}
    <section class="card details">
      <div class="card-head plain">
        <span>Details</span>
        <span class="sim-tag" class:live={usingRealSignal} class:warn={noSignal}>
          {usingRealSignal ? "live Wi-Fi" : noSignal ? "no signal" : "simulated"}
        </span>
      </div>
      {#if noSignal}
        <p class="empty">No signal source. Connect to a Wi-Fi network to read live strength.</p>
      {:else}
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
      {/if}
    </section>
  {/if}
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
        <span>How the scan works</span>
        <button class="icon-btn" aria-label="Close" onclick={() => (aboutOpen = false)}>✕</button>
      </div>

      <ol class="steps">
        <li><b>Hold the phone flat</b> and turn slowly in a full circle.</li>
        <li><b>Watch the dial.</b> The arrow always points toward the strongest direction; the big word and bars show how strong it is where you're pointing.</li>
        <li><b>Face it.</b> Turn until the arrow is at the top and it reads <i>You're facing it</i> — a short buzz confirms the lock.</li>
        <li><b>Save spots.</b> Tap <b>Save this spot</b> in a few places; the list ranks them strongest-first and stars the best — handy for placing a router or picking a seat.</li>
      </ol>

      <div class="legend">
        <div class="leg"><span class="dotc good"></span>Strong / Excellent</div>
        <div class="leg"><span class="dotc fair"></span>Fair</div>
        <div class="leg"><span class="dotc weak"></span>Weak</div>
      </div>

      <div class="callout">
        <b>What's real, what's not.</b>
        <p><b>Real:</b> the compass heading (phone magnetometer) and — when you're connected to Wi-Fi on Android — the <b>signal strength in dBm</b>, read live from the system. The dial then points to whichever direction you measured the strongest signal. The finder reads <b>Live Wi-Fi</b> when this is active.</p>
        <p><b>Estimated:</b> cellular dBm, ping and speed are derived from the signal level, not separate measurements.</p>
        <p><b>No signal?</b> With no Wi-Fi connection the app says so plainly and waits — it won't invent a reading. (On desktop it shows a labelled demo so you can preview the interaction.)</p>
      </div>

      <div class="ver">Signalume · prototype</div>
    </div>
  </div>
{/if}

<!-- ── SETTINGS SHEET ────────────────────────────────────────────────── -->
{#if settingsOpen}
  <div class="sheet-backdrop" data-theme={settings.theme} style={themeVars(settings.theme)}
       onclick={() => (settingsOpen = false)} role="presentation">
    <div class="sheet" onclick={(e) => e.stopPropagation()} role="dialog" aria-label="Settings">
      <div class="sheet-head">
        <span>Settings</span>
        <button class="icon-btn" aria-label="Close" onclick={() => (settingsOpen = false)}>✕</button>
      </div>

      <!-- ░ APPEARANCE ░ -->
      <div class="acc" class:open={openSection === "appearance"}>
        <button class="acc-head" onclick={() => toggleSection("appearance")}>
          <span class="acc-t">Appearance</span>
          <span class="acc-meta">{THEMES[settings.theme].label}</span>
          <span class="chev">▾</span>
        </button>
        {#if openSection === "appearance"}
          <div class="acc-body">
            {#each THEME_GROUPS as group}
              <div class="theme-group-label">{group}</div>
              <div class="themes">
                {#each Object.entries(THEMES).filter(([, t]) => t.group === group) as [id, t]}
                  <button class="theme-chip" class:on={settings.theme === id}
                          onclick={() => (settings.theme = id as ThemeId)}
                          title={t.blurb}>
                    <span class="sw-preview" style={themeVars(id as ThemeId)}>
                      <span class="sw-bg"></span>
                      <span class="sw-accent"></span>
                    </span>
                    <span class="sw-label">{t.label}</span>
                  </button>
                {/each}
              </div>
            {/each}
            <p class="set-note">{THEMES[settings.theme].blurb}.</p>
          </div>
        {/if}
      </div>

      <!-- ░ DISPLAY (feature visibility) ░ -->
      <div class="acc" class:open={openSection === "display"}>
        <button class="acc-head" onclick={() => toggleSection("display")}>
          <span class="acc-t">Display</span>
          <span class="acc-meta">Show / hide</span>
          <span class="chev">▾</span>
        </button>
        {#if openSection === "display"}
          <div class="acc-body">
            <button class="toggle" onclick={() => (settings.showSpots = !settings.showSpots)}>
              <span><b>Saved spots</b><small>The list of pinned places below the dial</small></span>
              <span class="sw-track" class:on={settings.showSpots}><span class="knob"></span></span>
            </button>
            <button class="toggle" onclick={() => (settings.showDetails = !settings.showDetails)}>
              <span><b>Technical details</b><small>Wi-Fi / cellular / ping numbers (simulated)</small></span>
              <span class="sw-track" class:on={settings.showDetails}><span class="knob"></span></span>
            </button>
            <button class="toggle" onclick={() => (settings.showReadout = !settings.showReadout)}>
              <span><b>Dial readout</b><small>The “% · 184° N” line in the centre</small></span>
              <span class="sw-track" class:on={settings.showReadout}><span class="knob"></span></span>
            </button>
            <button class="toggle" onclick={() => (settings.showGraticule = !settings.showGraticule)}>
              <span><b>Dial graticule</b><small>Faint rings and crosshairs behind the arrow</small></span>
              <span class="sw-track" class:on={settings.showGraticule}><span class="knob"></span></span>
            </button>
            <button class="toggle" onclick={() => (settings.showHint = !settings.showHint)}>
              <span><b>Helper text</b><small>The short tip under the Save button</small></span>
              <span class="sw-track" class:on={settings.showHint}><span class="knob"></span></span>
            </button>
          </div>
        {/if}
      </div>

      <!-- ░ BEHAVIOUR ░ -->
      <div class="acc" class:open={openSection === "behaviour"}>
        <button class="acc-head" onclick={() => toggleSection("behaviour")}>
          <span class="acc-t">Behaviour</span>
          <span class="acc-meta">Compass &amp; feedback</span>
          <span class="chev">▾</span>
        </button>
        {#if openSection === "behaviour"}
          <div class="acc-body">
            <div class="set-sub">Steadiness</div>
            <div class="seg-pick">
              {#each [["responsive","Responsive"],["balanced","Balanced"],["steady","Steady"]] as [id, label]}
                <button class="seg-opt" class:on={settings.smoothing === id}
                        onclick={() => (settings.smoothing = id as SmoothId)}>{label}</button>
              {/each}
            </div>
            <p class="set-note">How much the compass is smoothed. Pick <b>Steady</b> if the numbers jump around too much.</p>

            <button class="toggle" onclick={() => (settings.buzzOnLock = !settings.buzzOnLock)}>
              <span><b>Buzz when aligned</b><small>Short vibration the instant you face the strongest signal</small></span>
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
        {/if}
      </div>

      <!-- ░ DATA ░ -->
      <div class="acc" class:open={openSection === "data"}>
        <button class="acc-head" onclick={() => toggleSection("data")}>
          <span class="acc-t">Data &amp; scan</span>
          <span class="acc-meta">{pins.length} saved</span>
          <span class="chev">▾</span>
        </button>
        {#if openSection === "data"}
          <div class="acc-body">
            <p class="set-note">
              The compass is real; signal strength is a <b>simulated</b> field for
              now. <button class="link" onclick={() => { settingsOpen = false; aboutOpen = true; }}>See how the scan works</button>.
            </p>
            <button class="danger" onclick={() => clearPins()}>Clear saved spots</button>
            <button class="danger soft" onclick={() => { settings = { ...DEFAULTS }; showToast("Settings reset"); }}>
              Reset settings to defaults
            </button>
          </div>
        {/if}
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
  :global(body) { min-height: 100dvh; transition: background 0.3s ease; }

  .app {
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
    transition: background 0.3s ease, color 0.3s ease;
  }

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
    font: inherit; font-size: 12px; color: var(--muted);
    background: var(--surface); border: 1px solid var(--line);
    padding: 6px 11px; border-radius: 999px; cursor: pointer;
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
  .finder { display: flex; flex-direction: column; align-items: center; gap: 14px; padding: 16px 18px 22px; }
  .finder-head { display: flex; align-items: center; justify-content: space-between; width: 100%; }
  .scan-state { display: inline-flex; align-items: center; gap: 7px; font-size: 11px; color: var(--muted); letter-spacing: 0.3px; font-variant-numeric: tabular-nums; }
  .scan-state.live { color: var(--good); }
  .scan-state.warn { color: var(--weak); }
  .scan-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--accent); animation: pulse 1.6s ease-in-out infinite; }
  .scan-state.live .scan-dot { background: var(--good); }
  .scan-state.warn .scan-dot { background: var(--weak); animation: none; }
  .dial-wrap { position: relative; width: 236px; height: 236px; }
  .dial { display: block; }
  .dial-center {
    position: absolute; inset: 0; pointer-events: none;
    display: flex; flex-direction: column; align-items: center; justify-content: center;
  }
  .q-cap { font-size: 10px; letter-spacing: 3px; color: var(--muted); }
  .q-label { font-size: 30px; font-weight: 700; line-height: 1; margin-top: 4px; letter-spacing: 0.2px; }
  .q-label.none { font-size: 24px; color: var(--muted); }
  .q-sub { font-size: 11px; color: var(--muted); margin-top: 8px; text-align: center; max-width: 150px; line-height: 1.4; }
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
  .guide.warn { background: color-mix(in srgb, var(--weak) 10%, var(--surface-2)); border-color: color-mix(in srgb, var(--weak) 30%, transparent); }
  .guide-arrow { font-size: 26px; line-height: 1; color: var(--accent); flex-shrink: 0; width: 28px; text-align: center; }
  .guide.aligned .guide-arrow { color: var(--good); }
  .guide.warn .guide-arrow { color: var(--weak); }
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
  .primary-btn:disabled { background: var(--surface-2); color: var(--muted); cursor: default; transform: none; filter: none; }
  .hint { margin: 0; font-size: 11.5px; color: var(--muted); text-align: center; line-height: 1.55; max-width: 290px; }
  .hint b { color: var(--text); }

  /* ── Card heads / spots ── */
  .card-head { display: flex; justify-content: space-between; align-items: center; padding: 13px 16px; font-size: 13px; font-weight: 600; }
  .card-head.plain { padding-bottom: 0; }
  .sim-tag { font-size: 10px; font-weight: 600; letter-spacing: 0.6px; text-transform: uppercase; color: var(--muted); background: color-mix(in srgb, var(--text) 7%, transparent); padding: 2px 7px; border-radius: 6px; }
  .sim-tag.live { color: var(--good); background: color-mix(in srgb, var(--good) 15%, transparent); }
  .sim-tag.warn { color: var(--weak); background: color-mix(in srgb, var(--weak) 15%, transparent); }
  .link { font: inherit; font-size: 12px; color: var(--accent); background: none; border: none; cursor: pointer; padding: 0; }
  .link.tiny { font-size: 11px; }
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
    background: var(--surface-2); color: var(--text);
    border: 1px solid var(--line);
    padding: 10px 16px; border-radius: 999px; font-size: 12.5px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.3);
    animation: toast-in 0.2s ease;
  }
  @keyframes toast-in { from { opacity: 0; transform: translate(-50%, 6px); } to { opacity: 1; transform: translate(-50%, 0); } }

  /* ── Sheets (settings + about) ── */
  .sheet-backdrop { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: flex-end; justify-content: center; z-index: 50; }
  .sheet {
    width: 100%; max-width: 480px; max-height: 88dvh; overflow-y: auto;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--line); border-bottom: none;
    border-radius: 20px 20px 0 0;
    padding: 16px 18px calc(22px + env(safe-area-inset-bottom));
    display: flex; flex-direction: column; gap: 14px;
    font-family: ui-sans-serif, system-ui, sans-serif;
    animation: rise 0.22s cubic-bezier(0.22,1,0.36,1);
  }
  @keyframes rise { from { transform: translateY(20px); opacity: 0.6; } to { transform: translateY(0); opacity: 1; } }
  .sheet-head { display: flex; justify-content: space-between; align-items: center; font-size: 16px; font-weight: 600; }
  .set-note { margin: 0; font-size: 11.5px; color: var(--muted); line-height: 1.5; }
  .set-note b { color: var(--text); }
  .set-sub { font-size: 11px; letter-spacing: 1px; text-transform: uppercase; color: var(--muted); }

  /* ── Accordion sections ── */
  .acc { border: 1px solid var(--line); border-radius: 14px; background: var(--surface); overflow: hidden; }
  .acc.open { background: var(--surface); border-color: color-mix(in srgb, var(--accent) 30%, var(--line)); }
  .acc-head {
    width: 100%; display: flex; align-items: center; gap: 10px;
    font: inherit; text-align: left; color: var(--text);
    background: none; border: none; padding: 14px 16px; cursor: pointer;
  }
  .acc-t { font-size: 14px; font-weight: 600; flex: 1; }
  .acc-meta { font-size: 11px; color: var(--muted); }
  .chev { font-size: 11px; color: var(--muted); transition: transform 0.2s ease; }
  .acc.open .chev { transform: rotate(180deg); color: var(--accent); }
  .acc-body { padding: 4px 16px 16px; display: flex; flex-direction: column; gap: 10px; }

  /* ── Theme picker ── */
  .theme-group-label { font-size: 10px; letter-spacing: 1px; text-transform: uppercase; color: var(--muted); opacity: 0.8; margin-top: 4px; }
  .themes { display: grid; grid-template-columns: repeat(2, 1fr); gap: 8px; }
  .theme-chip {
    display: flex; align-items: center; gap: 9px;
    font: inherit; font-size: 13px; color: var(--text);
    background: var(--surface); border: 1px solid var(--line);
    padding: 9px 10px; border-radius: 11px; cursor: pointer; text-align: left;
  }
  .theme-chip.on { border-color: var(--accent); box-shadow: inset 0 0 0 1px var(--accent); }
  /* Only the little swatch paints with the candidate theme's own vars (set
     inline per swatch); the label keeps the current app theme so it stays
     readable on the sheet. */
  .sw-preview { position: relative; width: 26px; height: 26px; border-radius: 7px; overflow: hidden; flex-shrink: 0; border: 1px solid var(--line); }
  .sw-bg { position: absolute; inset: 0; background: var(--bg); }
  .sw-accent { position: absolute; right: 3px; bottom: 3px; top: 3px; width: 8px; border-radius: 4px; background: var(--accent); }
  .sw-label { color: var(--text); }

  /* ── Segmented + toggles ── */
  .seg-pick { display: flex; gap: 6px; background: var(--surface-2); border: 1px solid var(--line); border-radius: 12px; padding: 4px; }
  .seg-opt { flex: 1; font: inherit; font-size: 13px; color: var(--muted); background: none; border: none; padding: 9px 4px; border-radius: 9px; cursor: pointer; }
  .seg-opt.on { color: var(--text); background: var(--surface); }
  .toggle {
    display: flex; align-items: center; justify-content: space-between; gap: 12px;
    font: inherit; text-align: left; color: var(--text);
    background: var(--surface-2); border: 1px solid var(--line);
    padding: 12px 14px; border-radius: 12px; cursor: pointer;
  }
  .toggle b { display: block; font-size: 14px; font-weight: 600; }
  .toggle small { display: block; font-size: 11px; color: var(--muted); margin-top: 2px; }
  .sw-track { width: 42px; height: 24px; border-radius: 999px; background: var(--surface); border: 1px solid var(--line); position: relative; flex-shrink: 0; transition: background 0.2s; }
  .sw-track.on { background: var(--accent); }
  .knob { position: absolute; top: 2px; left: 2px; width: 18px; height: 18px; border-radius: 50%; background: var(--bg); transition: transform 0.2s; }
  .sw-track.on .knob { transform: translateX(18px); background: var(--bg); }

  .danger {
    font: inherit; font-size: 14px; color: var(--weak);
    background: var(--surface-2); border: 1px solid color-mix(in srgb, var(--weak) 30%, var(--line));
    padding: 12px; border-radius: 12px; cursor: pointer;
  }
  .danger.soft { color: var(--muted); border-color: var(--line); }

  /* ── About / scan explainer ── */
  .steps { margin: 0; padding-left: 18px; display: flex; flex-direction: column; gap: 9px; }
  .steps li { font-size: 12.5px; color: var(--muted); line-height: 1.5; }
  .steps b { color: var(--text); }
  .steps i { color: var(--accent); font-style: normal; }
  .legend { display: flex; gap: 16px; padding: 4px 2px; }
  .leg { display: inline-flex; align-items: center; gap: 6px; font-size: 11.5px; color: var(--muted); }
  .dotc { width: 9px; height: 9px; border-radius: 50%; }
  .dotc.good { background: var(--good); } .dotc.fair { background: var(--fair); } .dotc.weak { background: var(--weak); }
  .callout { background: var(--surface); border: 1px solid var(--line); border-radius: 12px; padding: 13px 14px; }
  .callout > b { font-size: 13px; }
  .callout p { margin: 8px 0 0; font-size: 12px; color: var(--muted); line-height: 1.55; }
  .callout p b { color: var(--text); }

  .ver { text-align: center; font-size: 11px; color: var(--muted); opacity: 0.7; }
</style>
