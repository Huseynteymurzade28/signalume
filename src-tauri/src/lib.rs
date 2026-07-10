use serde::Serialize;

#[derive(Serialize, Default)]
pub struct SignalReading {
    wifi_rssi: i32,
    wifi_connected: bool,
    cell_rssi: i32,
    cell_connected: bool,
}

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
                                }
                            }
                        }
                    }
                }
            }
            if env.exception_check().unwrap_or(false) { let _ = env.exception_clear(); }
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

#[tauri::command]
fn signal_reading() -> Result<SignalReading, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(read_signals))
        .unwrap_or_else(|_| Err("signal read panicked".into()))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![signal_reading])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
