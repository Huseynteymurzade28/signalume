use serde::Serialize;

#[derive(Serialize, Default)]
pub struct SignalReading {
    wifi_rssi: i32,
    wifi_connected: bool,
    wifi_link_mbps: i32, // negotiated link rate, 0 when unknown — not throughput
    cell_rssi: i32,
    cell_connected: bool,
    location_granted: bool, // getAllCellInfo needs runtime ACCESS_FINE_LOCATION
}

#[cfg(target_os = "android")]
const FINE_LOCATION: &str = "android.permission.ACCESS_FINE_LOCATION";

// ── Android: read Wi-Fi RSSI + cellular RSSI in one JNI session ───────────
#[cfg(target_os = "android")]
fn read_signals() -> Result<SignalReading, String> {
    use jni::objects::{JObject, JValue};
    use tao::platform::android::prelude::main_android_context;

    let ctx = main_android_context().ok_or_else(|| "android context not ready".to_string())?;
    let vm = unsafe { jni::JavaVM::from_raw(ctx.java_vm.cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread_as_daemon().map_err(|e| e.to_string())?;
    let context = unsafe { JObject::from_raw(ctx.context_jobject.cast()) };

    let result = (|| -> Result<SignalReading, String> {
        let mut reading = SignalReading::default();

        // ── Wi-Fi (connected AP RSSI) ──────────────────────────────────
        {
            let svc = env.new_string("wifi").map_err(|e| e.to_string())?;
            if let Ok(mgr) = env
                .call_method(&context, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&svc).into()])
                .and_then(|v| v.l())
            {
                if !mgr.is_null() {
                    if let Ok(info) = env
                        .call_method(&mgr, "getConnectionInfo", "()Landroid/net/wifi/WifiInfo;", &[])
                        .and_then(|v| v.l())
                    {
                        if !info.is_null() {
                            if let Ok(rssi) = env
                                .call_method(&info, "getRssi", "()I", &[])
                                .and_then(|v| v.i())
                            {
                                // Android sentinel for "no connection" is -127.
                                if rssi < 0 && rssi > -100 {
                                    reading.wifi_rssi = rssi;
                                    reading.wifi_connected = true;
                                    // LINK_SPEED_UNKNOWN is -1.
                                    let mbps = env
                                        .call_method(&info, "getLinkSpeed", "()I", &[])
                                        .and_then(|v| v.i())
                                        .unwrap_or(-1);
                                    reading.wifi_link_mbps = mbps.max(0);
                                }
                            }
                        }
                    }
                }
            }
            if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
        }

        // ── Location permission (gates the cellular read below) ────────
        {
            let perm = env.new_string(FINE_LOCATION).map_err(|e| e.to_string())?;
            let granted = env
                .call_method(&context, "checkSelfPermission", "(Ljava/lang/String;)I", &[(&perm).into()])
                .and_then(|v| v.i())
                .map(|r| r == 0) // PERMISSION_GRANTED
                .unwrap_or(false);
            if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
            reading.location_granted = granted;
        }

        // ── Cellular (serving cell dBm via getAllCellInfo) ─────────────
        // ACCESS_FINE_LOCATION covers the permission requirement on API 29+.
        // On older APIs the call may throw SecurityException — we clear it
        // and leave cell_connected false rather than crashing.
        {
            let svc = env.new_string("phone").map_err(|e| e.to_string())?;
            if let Ok(tm) = env
                .call_method(&context, "getSystemService", "(Ljava/lang/String;)Ljava/lang/Object;", &[(&svc).into()])
                .and_then(|v| v.l())
            {
                if !tm.is_null() {
                    let cells_res = env
                        .call_method(&tm, "getAllCellInfo", "()Ljava/util/List;", &[])
                        .and_then(|v| v.l());
                    if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }

                    if let Ok(cells) = cells_res {
                        if !cells.is_null() {
                            let size = env
                                .call_method(&cells, "size", "()I", &[])
                                .and_then(|v| v.i())
                                .unwrap_or(0);

                            // Walk cells; pick the registered one with the strongest dBm.
                            let mut best_dbm = i32::MIN;
                            'cells: for i in 0..size {
                                let cell_res = env
                                    .call_method(&cells, "get", "(I)Ljava/lang/Object;", &[JValue::Int(i)])
                                    .and_then(|v| v.l());
                                if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); break 'cells; }
                                let cell = match cell_res { Ok(c) if !c.is_null() => c, _ => continue };

                                let registered = env
                                    .call_method(&cell, "isRegistered", "()Z", &[])
                                    .and_then(|v| v.z())
                                    .unwrap_or(false);
                                if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
                                if !registered { continue; }

                                let sig_res = env
                                    .call_method(&cell, "getCellSignalStrength", "()Landroid/telephony/CellSignalStrength;", &[])
                                    .and_then(|v| v.l());
                                if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); continue; }

                                if let Ok(sig) = sig_res {
                                    if !sig.is_null() {
                                        let dbm = env
                                            .call_method(&sig, "getDbm", "()I", &[])
                                            .and_then(|v| v.i())
                                            .unwrap_or(i32::MIN);
                                        if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
                                        // Valid dBm: negative, not the sentinel MIN.
                                        if dbm < 0 && dbm > best_dbm { best_dbm = dbm; }
                                    }
                                }
                            }

                            if best_dbm > i32::MIN {
                                reading.cell_rssi = best_dbm;
                                reading.cell_connected = true;
                            }
                        }
                    }
                }
            }
            if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
        }

        Ok(reading)
    })();

    if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
    result
}

#[cfg(not(target_os = "android"))]
fn read_signals() -> Result<SignalReading, String> {
    Err("signal reading is only available on Android".into())
}

// ── Android: show the system location-permission dialog ───────────────
// The result lands in Activity.onRequestPermissionsResult, which we don't
// hook — the frontend just sees `location_granted` flip on the next poll.
#[cfg(target_os = "android")]
fn request_location() -> Result<(), String> {
    use jni::objects::JObject;
    use tao::platform::android::prelude::main_android_context;

    let ctx = main_android_context().ok_or_else(|| "android context not ready".to_string())?;
    let vm = unsafe { jni::JavaVM::from_raw(ctx.java_vm.cast()) }.map_err(|e| e.to_string())?;
    let mut env = vm.attach_current_thread_as_daemon().map_err(|e| e.to_string())?;
    let activity = unsafe { JObject::from_raw(ctx.context_jobject.cast()) };

    let result = (|| -> Result<(), String> {
        let string_cls = env.find_class("java/lang/String").map_err(|e| e.to_string())?;
        let perms = env.new_object_array(2, &string_cls, JObject::null()).map_err(|e| e.to_string())?;
        let fine = env.new_string(FINE_LOCATION).map_err(|e| e.to_string())?;
        let coarse = env.new_string("android.permission.ACCESS_COARSE_LOCATION").map_err(|e| e.to_string())?;
        env.set_object_array_element(&perms, 0, &fine).map_err(|e| e.to_string())?;
        env.set_object_array_element(&perms, 1, &coarse).map_err(|e| e.to_string())?;
        env.call_method(&activity, "requestPermissions", "([Ljava/lang/String;I)V", &[(&perms).into(), 4711.into()])
            .map_err(|e| e.to_string())?;
        Ok(())
    })();

    if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
    result
}

#[cfg(not(target_os = "android"))]
fn request_location() -> Result<(), String> {
    Ok(())
}

#[tauri::command]
fn request_location_permission() -> Result<(), String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(request_location))
        .unwrap_or_else(|_| Err("permission request panicked".into()))
}

// Real round-trip latency: time a TCP handshake to a public anycast resolver.
// No ICMP (needs root on Android) and no HTTP payload — just connect + drop.
#[tauri::command]
async fn ping_ms() -> Option<u32> {
    tauri::async_runtime::spawn_blocking(|| {
        use std::net::{SocketAddr, TcpStream};
        use std::time::{Duration, Instant};
        for addr in ["1.1.1.1:443", "8.8.8.8:443"] {
            let addr: SocketAddr = addr.parse().ok()?;
            let start = Instant::now();
            if TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok() {
                return Some(start.elapsed().as_millis() as u32);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}

#[tauri::command]
fn signal_reading() -> Result<SignalReading, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(read_signals))
        .unwrap_or_else(|_| Err("signal read panicked".into()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![signal_reading, request_location_permission, ping_ms])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
