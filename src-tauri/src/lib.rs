#[macro_use]
mod account;
#[macro_use]
mod device;
#[macro_use]
mod sideload;
#[macro_use]
mod pairing;
mod operation;

use crate::{
    account::{
        cleanup_all, delete_account, delete_app_id, get_certificates, invalidate_account,
        list_app_ids, logged_in_as, login_email_pass, login_stored_pass, revoke_certificate,
    },
    device::{list_devices, set_selected_device, DeviceInfoMutex},
    pairing::{installed_pairing_apps, place_pairing_cmd},
    sideload::{install_sidestore_operation, sideload_operation},
};
use tauri::Manager;
use env_logger::Env;
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    INIT_LOGGER.call_once(|| {
        let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .setup(|app| {
            app.manage(DeviceInfoMutex::new(None));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            login_email_pass,
            invalidate_account,
            logged_in_as,
            login_stored_pass,
            delete_account,
            list_devices,
            sideload_operation,
            set_selected_device,
            install_sidestore_operation,
            get_certificates,
            revoke_certificate,
            list_app_ids,
            delete_app_id,
            cleanup_all,
            installed_pairing_apps,
            place_pairing_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
