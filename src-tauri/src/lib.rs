use serde::Serialize;

// What the frontend gets back from a Wi-Fi read.
#[derive(Serialize, Default)]
pub struct WifiReading {
    /// Received signal strength in dBm (e.g. -55). 0 when unavailable.
    rssi: i32,
    /// Whether the device is associated with a Wi-Fi network right now.
    connected: bool,
}

// ── Android: talk to android.net.wifi.WifiManager directly over JNI ───────
// We read the *connected* access point's RSSI, which updates continuously and
// — unlike a full scan — isn't subject to Android's scan throttling. That's
// exactly what a "turn until the signal is strongest" finder wants.
#[cfg(target_os = "android")]
fn read_wifi() -> Result<WifiReading, String> {
    use jni::objects::JObject;
    use tao::platform::android::prelude::main_android_context;

    // Live JavaVM + Activity handle, populated by tao when the app starts.
    let ctx = main_android_context().ok_or_else(|| "android context not ready".to_string())?;
    let vm = unsafe { jni::JavaVM::from_raw(ctx.java_vm.cast()) }.map_err(|e| e.to_string())?;
    // Attach as a daemon: it stays attached (no detach on drop), which both
    // matches how tao itself attaches and sidesteps the detach-with-pending-
    // exception abort.
    let mut env = vm.attach_current_thread_as_daemon().map_err(|e| e.to_string())?;
    let context = unsafe { JObject::from_raw(ctx.context_jobject.cast()) };

    // Do the JNI work in an inner closure so we can ALWAYS clear any pending
    // Java exception before this thread detaches — otherwise the JVM aborts
    // the whole process ("JNI DETACH called with pending exception").
    let result = (|| -> Result<WifiReading, String> {
        // WifiManager wifi = (WifiManager) context.getSystemService("wifi");
        let service = env.new_string("wifi").map_err(|e| e.to_string())?;
        let wifi_manager = env
            .call_method(
                &context,
                "getSystemService",
                "(Ljava/lang/String;)Ljava/lang/Object;",
                &[(&service).into()],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;

        // WifiInfo info = wifi.getConnectionInfo();
        let info = env
            .call_method(
                &wifi_manager,
                "getConnectionInfo",
                "()Landroid/net/wifi/WifiInfo;",
                &[],
            )
            .and_then(|v| v.l())
            .map_err(|e| e.to_string())?;

        // No connection info at all → simply "not connected" (don't call into null).
        if info.is_null() {
            return Ok(WifiReading { rssi: 0, connected: false });
        }

        // int rssi = info.getRssi();
        let rssi = env
            .call_method(&info, "getRssi", "()I", &[])
            .and_then(|v| v.i())
            .map_err(|e| e.to_string())?;

        // When disconnected, Android reports a sentinel like -127.
        let connected = rssi < 0 && rssi > -100;
        Ok(WifiReading { rssi, connected })
    })();

    if env.exception_check().unwrap_or(false) {
        let _ = env.exception_clear();
    }
    result
}

#[cfg(not(target_os = "android"))]
fn read_wifi() -> Result<WifiReading, String> {
    Err("wifi rssi is only available on Android".into())
}

#[tauri::command]
fn wifi_reading() -> Result<WifiReading, String> {
    // Safety net: a panic crossing the JNI/IPC boundary aborts the whole app,
    // so contain any unexpected panic and surface it as a normal error instead.
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(read_wifi))
        .unwrap_or_else(|_| Err("wifi read panicked".into()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![wifi_reading])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
