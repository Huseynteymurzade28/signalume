# 📡 Signalume

A cyber-compass and signal detector mobile app with a Tokyo Night / retro-cyberpunk aesthetic.

Built with **SvelteKit (Svelte 5)**, **Tauri v2**, and a fully client-side UI. Currently a
**prototype**: the radar, compass heading, signal metrics, and terminal log stream are all
simulated on the frontend (mock data + intervals) — no live backend or network calls required.

## ✨ Features

- 🧭 **Cyber-Compass & Radar** — a continuously sweeping HTML Canvas radar that spawns
  temporary neon blips, with a live simulated heading in the center.
- 📊 **Live Signal Metrics** — animated Wi-Fi / Cellular strength bars plus ping & throughput
  readouts that jitter realistically.
- 📜 **Terminal Log Stream** — an auto-scrolling terminal panel pushing mock system logs.

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
