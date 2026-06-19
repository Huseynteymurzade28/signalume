# 📡 Signalume

A directional signal finder: turn the phone slowly and an arrow points toward the
strongest signal, so you can find the best spot to stand (or place a router).

Built with **SvelteKit (Svelte 5)** + **Tauri v2** (Android target).

- **Compass heading** is real — read from the phone's magnetometer.
- **Wi-Fi signal strength (dBm)** is real on Android when you're connected to a
  network — read live from the system `WifiManager` over JNI (`src-tauri/src/lib.rs`).
  As you turn, the app keeps the strongest RSSI per heading and points the arrow that way.
- With no Wi-Fi connection (or on desktop) it falls back to a clearly-labelled **demo**
  signal field so the interaction is still visible. Cellular / ping / speed are estimates
  derived from the signal level, not separate measurements.

## ✨ Features

- 🧭 **Directional finder** — an instrument-style dial with degree ticks; one arrow always
  points to the strongest measured direction, with plain-language "turn left / right / you're
  facing it" guidance.
- 📍 **Saved spots** — pin places as you walk; the list ranks them strongest-first and stars the best.
- 🎨 **8 field-instrument themes** — Beacon, Tide, Phosphor, Blueprint, Ranger, Night Ops, Paper, Graphite.
- ⚙️ **Detailed settings** — collapsible sections for appearance, display (show/hide each element),
  behaviour (smoothing, haptics, keep-awake) and data.

## 🛠️ Tech Stack

- [SvelteKit](https://svelte.dev/) (Svelte 5, static SPA via `@sveltejs/adapter-static`)
- [Tauri v2](https://v2.tauri.app/) (Android target)
- TypeScript + Vite

> The app builds as a **static SPA** (`ssr = false`, `prerender = true`) because the Tauri
> WebView has no Node.js server — it serves prebuilt files from `tauri.localhost`.

## 🚀 Development

```bash
# install deps
npm install

# run in the browser (desktop dev)
npm run dev

# build the static frontend
npm run build
```

### 📱 Android (standalone build, no dev server)

```bash
npm run build
npx tauri android build --apk --debug
adb install -r src-tauri/gen/android/app/build/outputs/apk/universal/debug/app-universal-debug.apk
```

## 📄 License

MIT
