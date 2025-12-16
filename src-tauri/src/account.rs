use isideload::{
    developer_session::{DeveloperDeviceType, DeveloperSession, ListAppIdsResponse},
    AnisetteConfiguration, AppleAccount,
};
use keyring::Entry;
use once_cell::sync::OnceCell;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    sync::{mpsc::RecvTimeoutError, Arc, Mutex},
    time::Duration,
};
use tauri::{AppHandle, Emitter, Listener, Manager, Window};
use tauri_plugin_store::StoreExt;
use log::{error, warn, info, debug};

pub static APPLE_ACCOUNT: OnceCell<Mutex<Option<Arc<AppleAccount>>>> = OnceCell::new();

#[tauri::command]
pub async fn login_email_pass(
    handle: AppHandle,
    window: Window,
    email: String,
    password: String,
    anisette_server: String,
    save_credentials: bool,
) -> Result<String, String> {
    let cell = APPLE_ACCOUNT.get_or_init(|| Mutex::new(None));
    let account = login(&handle, &window, email, password.clone(), anisette_server).await?;
    let mut account_guard = cell.lock().unwrap();
    *account_guard = Some(account.clone());

    if save_credentials {
        let pass_entry = Entry::new("iloader", &account.apple_id)
            .map_err(|e| format!("Failed to create keyring entry for credentials: {:?}.", e))?;
        pass_entry
            .set_password(&password)
            .map_err(|e| format!("Failed to save credentials to keyring: {:?}", e))?;
        let store = handle
            .store("data.json")
            .map_err(|e| format!("Failed to get store: {:?}", e))?;
        let mut existing_ids = store
            .get("ids")
            .unwrap_or_else(|| Value::Array(vec![]))
            .as_array()
            .cloned()
            .unwrap_or_else(std::vec::Vec::new);
        let value = Value::String(account.apple_id.clone());
        if !existing_ids.contains(&value) {
            existing_ids.push(value);
        }
        store.set("ids", Value::Array(existing_ids));
    }
    Ok(account.apple_id.clone())
}

#[tauri::command]
pub async fn login_stored_pass(
    handle: AppHandle,
    window: Window,
    email: String,
    anisette_server: String,
) -> Result<String, String> {
    let cell = APPLE_ACCOUNT.get_or_init(|| Mutex::new(None));
    let pass_entry = Entry::new("iloader", &email)
        .map_err(|e| format!("Failed to create keyring entry for credentials: {:?}.", e))?;
    let password = pass_entry
        .get_password()
        .map_err(|e| format!("Failed to get credentials: {:?}", e))?;
    let account = login(&handle, &window, email, password, anisette_server).await?;
    let mut account_guard = cell.lock().unwrap();
    *account_guard = Some(account.clone());

    Ok(account.apple_id.clone())
}

#[tauri::command]
pub fn delete_account(handle: AppHandle, email: String) -> Result<(), String> {
    let pass_entry = Entry::new("iloader", &email)
        .map_err(|e| format!("Failed to create keyring entry for credentials: {:?}.", e))?;
    pass_entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete credentials: {:?}", e))?;
    let store = handle
        .store("data.json")
        .map_err(|e| format!("Failed to get store: {:?}", e))?;
    let mut existing_ids = store
        .get("ids")
        .unwrap_or_else(|| Value::Array(vec![]))
        .as_array()
        .cloned()
        .unwrap_or_else(std::vec::Vec::new);
    existing_ids.retain(|v| v.as_str().is_none_or(|s| s != email));
    store.set("ids", Value::Array(existing_ids));
    Ok(())
}

#[tauri::command]
pub fn logged_in_as() -> Option<String> {
    let account = get_account();
    if let Ok(account) = account {
        return Some(account.apple_id.clone());
    }
    None
}

#[tauri::command]
pub fn invalidate_account() {
    let cell = APPLE_ACCOUNT.get();
    if let Some(account) = cell {
        let mut account_guard = account.lock().unwrap();
        *account_guard = None;
    }
}

pub fn get_account() -> Result<Arc<AppleAccount>, String> {
    let cell = APPLE_ACCOUNT.get_or_init(|| Mutex::new(None));
    {
        let account_guard = cell.lock().unwrap();
        if let Some(account) = &*account_guard {
            return Ok(account.clone());
        }
    }

    Err("Not logged in".to_string())
}

pub async fn get_developer_session() -> Result<DeveloperSession, String> {
    debug!("Getting developer session");
    let account = get_account().map_err(|e| {
        error!("No account available: {}", e);
        e
    })?;

    let mut dev_session = DeveloperSession::new(account);

    let teams = match dev_session.list_teams().await {
        Ok(t) => {
            info!("Successfully listed {} teams", t.len());
            t
        },
        Err(e) => {
            // This code means we have been logged in for too long and we must relogin again
            let is_22411 = match &e {
                isideload::Error::Auth(code, _) => *code == -22411,
                isideload::Error::DeveloperSession(code, _) => *code == -22411,
                _ => false,
            };
            if is_22411 {
                warn!("Session expired (error -22411), invalidating account");
                invalidate_account();
                return Err(format!("Session timed out, please try again: {:?}", e));
            } else {
                error!("Failed to list teams: {:?}", e);
                return Err(format!("Failed to list teams: {:?}", e));
            }
        }
    };

    if teams.is_empty() {
        warn!("No teams found for account");
        return Err("No developer teams found for this account".to_string());
    }

    dev_session.set_team(teams[0].clone());
    info!("Using team with ID: {}", teams[0].team_id);

    Ok(dev_session)
}

async fn login(
    handle: &AppHandle,
    window: &Window,
    email: String,
    password: String,
    anisette_server: String,
) -> Result<Arc<AppleAccount>, String> {
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let window_clone = window.clone();
    let tfa_closure = move || -> Result<String, String> {
        window_clone
            .emit("2fa-required", ())
            .expect("Failed to emit 2fa-required event");

        let tx = tx.clone();
        let handler_id = window_clone.listen("2fa-recieved", move |event| {
            let code = event.payload();
            let _ = tx.send(code.to_string());
        });

        let result = rx.recv_timeout(Duration::from_secs(120));
        window_clone.unlisten(handler_id);

        match result {
            Ok(code) => {
                let code = code.trim_matches('"').to_string();
                Ok(code)
            }
            Err(RecvTimeoutError::Timeout) => Err("2FA cancelled or timed out".to_string()),
            Err(RecvTimeoutError::Disconnected) => Err("2FA disconnected".to_string()),
        }
    };

    let config = AnisetteConfiguration::default();
    let config =
        config.set_configuration_path(handle.path().app_config_dir().map_err(|e| e.to_string())?);
    let config = config.set_anisette_url_v3(format!("https://{}", anisette_server));

    let account = AppleAccount::login(
        || Ok((email.clone().to_lowercase(), password.clone())),
        tfa_closure,
        config,
    )
    .await;
    if let Err(e) = account {
        return Err(e.to_string());
    }
    let account = Arc::new(account.unwrap());

    Ok(account)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CertificateInfo {
    pub name: String,
    pub certificate_id: String,
    pub serial_number: String,
    pub machine_name: String,
    #[serde(default)]
    pub machine_id: String,
}

#[tauri::command]
pub async fn get_certificates_cached(
    handle: AppHandle,
) -> Result<Vec<CertificateInfo>, String> {
    // Попытка получить из кэша
    if let Ok(store) = handle.store("cache.json") {
        if let Some(cached) = store.get("certificates") {
            if let Some(cached_time) = store.get("certificates_cache_time") {
                if let (Some(certs_json), Some(time_json)) = (cached.as_array(), cached_time.as_u64()) {
                    let cache_age = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .saturating_sub(time_json);

                    // Используем кэш если он не старше 5 минут
                    if cache_age < 300 {
                        if let Ok(certs) = serde_json::from_value::<Vec<CertificateInfo>>(Value::Array(certs_json.clone())) {
                            info!("Using cached certificates (age: {}s, count: {})", cache_age, certs.len());
                            return Ok(certs);
                        } else {
                            warn!("Failed to deserialize cached certificates, fetching fresh data");
                        }
                    } else {
                        debug!("Cache expired (age: {}s), fetching fresh data", cache_age);
                    }
                }
            }
        }
    }

    // Если кэш не работает, получаем свежие данные
    let certs = get_certificates().await?;

    // Сохраняем в кэш
    if let Ok(store) = handle.store("cache.json") {
        if let Ok(json) = serde_json::to_value(&certs) {
            let cache_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            store.set("certificates", json);
            store.set("certificates_cache_time", Value::Number(cache_time.into()));
            info!("Cached {} certificates", certs.len());
        }
    }

    Ok(certs)
}

#[tauri::command]
pub async fn get_certificates() -> Result<Vec<CertificateInfo>, String> {
    info!("Starting to fetch certificates from Apple Developer API");

    let dev_session = get_developer_session().await.map_err(|e| {
        error!("Failed to get developer session: {:?}", e);
        format!("Failed to get developer session: {:?}", e)
    })?;

    let team = dev_session
        .get_team()
        .await
        .map_err(|e| {
            error!("Failed to get developer team: {:?}", e);
            format!("Failed to get developer team: {:?}", e)
        })?;

    info!("Fetching development certificates for team: {:?}", team.team_id);

    // Попытка получить сертификаты с обработкой ошибок парсинга
    let certificates = match dev_session
        .list_all_development_certs(DeveloperDeviceType::Ios, &team)
        .await
    {
        Ok(certs) => {
            info!("Successfully fetched {} certificates from Apple API", certs.len());
            debug!("Certificate details: {:?}", certs.iter().map(|c| &c.name).collect::<Vec<_>>());
            certs
        },
        Err(e) => {
            let error_str = format!("{:?}", e);
            error!("Failed to fetch certificates: {}", error_str);

            // Если ошибка связана с парсингом machineId, попробуем более детальное сообщение
            if error_str.contains("machineId") || error_str.contains("Parse") || error_str.contains("machineld") {
                warn!("machineId parsing error detected - this may indicate an Apple API format change");
                return Err(format!(
                    "Failed to parse certificates from Apple API. This may be due to an API format change.\n\n\
                    Error details: {:?}\n\n\
                    Possible solutions:\n\
                    1. Log out and log back in to refresh your session\n\
                    2. Revoke all existing certificates and create new ones\n\
                    3. Check for updates to iloader\n\
                    4. Report this issue to the iloader developers with the error details",
                    e
                ));
            }
            return Err(format!("Failed to get development certificates: {:?}", e));
        }
    };

    let result: Vec<CertificateInfo> = certificates
        .into_iter()
        .map(|cert| {
            // Валидация и обработка данных сертификата
            let machine_id = cert.machine_id.trim().to_string();
            if machine_id.is_empty() {
                debug!("Certificate '{}' has empty machine_id", cert.name);
            }

            // Валидация обязательных полей
            let name = if cert.name.is_empty() {
                warn!("Certificate has empty name, using default");
                "Unknown Certificate".to_string()
            } else {
                cert.name
            };

            let certificate_id = if cert.certificate_id.is_empty() {
                warn!("Certificate '{}' has empty certificate_id", name);
                String::new()
            } else {
                cert.certificate_id
            };

            let serial_number = if cert.serial_number.is_empty() {
                warn!("Certificate '{}' has empty serial_number", name);
                String::new()
            } else {
                cert.serial_number
            };

            CertificateInfo {
                name,
                certificate_id,
                serial_number,
                machine_name: cert.machine_name,
                machine_id,
            }
        })
        .filter(|cert| {
            // Фильтруем невалидные сертификаты
            if cert.certificate_id.is_empty() || cert.serial_number.is_empty() {
                warn!("Filtering out invalid certificate: {}", cert.name);
                false
            } else {
                true
            }
        })
        .collect();

    info!("Successfully processed {} certificates", result.len());
    Ok(result)
}

#[tauri::command]
pub async fn revoke_certificate(serial_number: String) -> Result<(), String> {
    let dev_session = get_developer_session().await?;
    let team = dev_session
        .get_team()
        .await
        .map_err(|e| format!("Failed to get developer team: {:?}", e))?;
    dev_session
        .revoke_development_cert(DeveloperDeviceType::Ios, &team, &serial_number)
        .await
        .map_err(|e| format!("Failed to revoke development certificates: {:?}", e))?;
    Ok(())
}

#[tauri::command]
pub async fn list_app_ids() -> Result<ListAppIdsResponse, String> {
    let dev_session = get_developer_session().await?;
    let team = dev_session
        .get_team()
        .await
        .map_err(|e| format!("Failed to get developer team: {:?}", e))?;
    let app_ids = dev_session
        .list_app_ids(DeveloperDeviceType::Ios, &team)
        .await
        .map_err(|e| format!("Failed to list App IDs: {:?}", e))?;
    Ok(app_ids)
}

#[tauri::command]
pub async fn delete_app_id(app_id_id: String) -> Result<(), String> {
    let dev_session = get_developer_session().await?;
    let team = dev_session
        .get_team()
        .await
        .map_err(|e| format!("Failed to get developer team: {:?}", e))?;
    dev_session
        .delete_app_id(DeveloperDeviceType::Ios, &team, app_id_id)
        .await
        .map_err(|e| format!("Failed to delete App ID: {:?}", e))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResult {
    pub certificates_revoked: u32,
    pub app_ids_deleted: u32,
    pub errors: Vec<String>,
}

#[tauri::command]
pub async fn cleanup_all() -> Result<CleanupResult, String> {
    let dev_session = get_developer_session().await?;
    let team = dev_session
        .get_team()
        .await
        .map_err(|e| format!("Failed to get developer team: {:?}", e))?;

    let mut result = CleanupResult {
        certificates_revoked: 0,
        app_ids_deleted: 0,
        errors: Vec::new(),
    };

    // 撤销所有证书
    info!("Starting cleanup: fetching certificates to revoke");
    let certificates = match dev_session
        .list_all_development_certs(DeveloperDeviceType::Ios, &team)
        .await
    {
        Ok(certs) => {
            info!("Found {} certificates to revoke", certs.len());
            certs
        },
        Err(e) => {
            let error_str = format!("{:?}", e);
            error!("Failed to fetch certificates for cleanup: {}", error_str);

            // Обработка ошибок парсинга при получении сертификатов
            if error_str.contains("machineId") || error_str.contains("Parse") || error_str.contains("machineld") {
                warn!("machineId parsing error during cleanup - adding to errors list");
                result.errors.push(format!(
                    "Failed to fetch certificates for cleanup due to parsing error: {:?}. \
                    You may need to manually revoke certificates through Apple Developer Portal.",
                    e
                ));
                return Ok(result); // Возвращаем частичный результат
            }
            return Err(format!("Failed to get certificates: {:?}", e));
        }
    };

    for cert in certificates {
        info!("Revoking certificate: {} (serial: {})", cert.name, cert.serial_number);
        match dev_session
            .revoke_development_cert(DeveloperDeviceType::Ios, &team, &cert.serial_number)
            .await
        {
            Ok(_) => {
                result.certificates_revoked += 1;
                debug!("Successfully revoked certificate: {}", cert.name);
            },
            Err(e) => {
                error!("Failed to revoke certificate {}: {:?}", cert.name, e);
                result
                    .errors
                    .push(format!("Failed to revoke certificate {}: {:?}", cert.name, e));
            }
        }
    }
    info!("Revoked {} certificates", result.certificates_revoked);

    // 删除所有 App ID
    info!("Starting cleanup: fetching App IDs to delete");
    let app_ids_response = dev_session
        .list_app_ids(DeveloperDeviceType::Ios, &team)
        .await
        .map_err(|e| {
            error!("Failed to list App IDs: {:?}", e);
            format!("Failed to list App IDs: {:?}", e)
        })?;

    info!("Found {} App IDs to delete", app_ids_response.app_ids.len());

    for app_id in app_ids_response.app_ids {
        info!("Deleting App ID: {} ({})", app_id.name, app_id.identifier);
        match dev_session
            .delete_app_id(DeveloperDeviceType::Ios, &team, app_id.app_id_id.clone())
            .await
        {
            Ok(_) => {
                result.app_ids_deleted += 1;
                debug!("Successfully deleted App ID: {}", app_id.name);
            },
            Err(e) => {
                error!("Failed to delete App ID {}: {:?}", app_id.name, e);
                result
                    .errors
                    .push(format!("Failed to delete App ID {}: {:?}", app_id.name, e));
            }
        }
    }

    info!("Cleanup completed: {} certificates revoked, {} App IDs deleted, {} errors",
          result.certificates_revoked, result.app_ids_deleted, result.errors.len());
    Ok(result)
}
