use actix_web::{web, App, HttpResponse, HttpServer, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

static SERVER_RUNNING: AtomicBool = AtomicBool::new(false);

type ShareMap = Arc<Mutex<HashMap<String, ShareData>>>;

#[derive(Deserialize, Serialize, Clone)]
struct ShareData {
    encrypted: String,
    expiry: u64,
    use_count: u32,
    original_filename: String,
}

async fn share_handler(path: web::Path<String>, map: web::Data<ShareMap>) -> Result<HttpResponse> {
    let code = path.into_inner();
    let mut shares = map.lock().unwrap();

    if let Some(data) = shares.get_mut(&code) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if now > data.expiry {
            shares.remove(&code);
            return Ok(HttpResponse::NotFound().json("Expired or used"));
        }

        if data.use_count >= 1 {
            shares.remove(&code);
            return Ok(HttpResponse::NotFound().json("Already used"));
        }

        data.use_count += 1;

        let response = ShareResponse {
            encrypted: data.encrypted.clone(),
            original_filename: data.original_filename.clone(),
        };

        Ok(HttpResponse::Ok().json(response))
    } else {
        Ok(HttpResponse::NotFound().json("Invalid code"))
    }
}

#[derive(Serialize)]
struct ShareResponse {
    encrypted: String,
    original_filename: String,
}

#[derive(serde::Serialize)]
struct EncryptedFileResult {
    encrypted: String,
    share_code: String,
}

#[derive(serde::Serialize)]
struct DecryptedResult {
    decrypted: Vec<u8>,
}

#[tauri::command]
async fn decrypt_file(
    encrypted_base64: String,
    key_bytes: Vec<u8>,
) -> Result<DecryptedResult, String> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};

    let final_data = base64::engine::general_purpose::STANDARD
        .decode(&encrypted_base64)
        .map_err(|error| error.to_string())?;
    if final_data.len() < 12 {
        return Err("Invalid data".to_string());
    }

    let nonce_bytes: [u8; 12] = final_data[0..12]
        .try_into()
        .map_err(|_| "Nonce error".to_string())?;
    let mut in_out = final_data[12..].to_vec();

    let unbound_key =
        UnboundKey::new(&AES_256_GCM, &key_bytes).map_err(|error| error.to_string())?;
    let key = LessSafeKey::new(unbound_key);
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);
    let aad = Aad::empty();

    let decrypted = key
        .open_in_place(nonce, aad, &mut in_out)
        .map_err(|error| error.to_string())?;

    Ok(DecryptedResult {
        decrypted: decrypted.to_vec(),
    })
}

#[tauri::command]
async fn encrypt_file(
    path: String,
    key_bytes: Vec<u8>,
    share_map: tauri::State<'_, ShareMap>,
) -> Result<EncryptedFileResult, String> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    use ring::rand::{SecureRandom, SystemRandom};
    use std::fs::read;
    use uuid::Uuid;

    let data = read(&path).map_err(|error| error.to_string())?;
    let mut in_out = data.clone();

    let unbound_key =
        UnboundKey::new(&AES_256_GCM, &key_bytes).map_err(|error| error.to_string())?;

    let key = LessSafeKey::new(unbound_key);

    let rng = SystemRandom::new();

    let mut nonce_bytes = [0u8; 12];
    rng.fill(&mut nonce_bytes)
        .map_err(|error| error.to_string())?;
    let nonce = Nonce::assume_unique_for_key(nonce_bytes);

    let aad = Aad::empty();

    key.seal_in_place_append_tag(nonce, aad, &mut in_out)
        .map_err(|error| error.to_string())?;

    let mut final_data = vec![];
    final_data.extend_from_slice(&nonce_bytes);
    final_data.extend_from_slice(&in_out);

    let encrypted_base64 = base64::engine::general_purpose::STANDARD.encode(&final_data);
    let share_code = Uuid::new_v4().to_string();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let original_filename = std::path::Path::new(&path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or("file")
        .to_string();

    let share_data = ShareData {
        encrypted: encrypted_base64.clone(),
        expiry: now + 3600,
        use_count: 0,
        original_filename,
    };

    {
        let mut map = share_map.lock().unwrap();
        map.insert(share_code.clone(), share_data);
    }

    Ok(EncryptedFileResult {
        encrypted: encrypted_base64,
        share_code,
    })
}

#[tauri::command]
async fn get_local_ip() -> Result<String, String> {
    use std::net::{SocketAddr, TcpStream};

    let socket = SocketAddr::from(([8, 8, 8, 8], 53));
    match TcpStream::connect_timeout(&socket, std::time::Duration::from_secs(5)) {
        Ok(stream) => {
            if let Ok(addr) = stream.local_addr() {
                return Ok(addr.ip().to_string());
            }
        }
        Err(_) => {}
    }

    Ok("127.0.0.1".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

    let share_map_for_server = share_map.clone();

    if SERVER_RUNNING
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        tauri::async_runtime::spawn(async move {
            let ip = IpAddr::V4(Ipv4Addr::UNSPECIFIED);
            let addr = format!("{}:8080", ip);

            println!("HTTP Server Running on http://{}", addr);

            if let Err(e) = HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(share_map_for_server.clone()))
                    .route("/share/{code}", web::get().to(share_handler))
            })
            .bind("127.0.0.1:8080")
            .unwrap()
            .run()
            .await
            {
                eprintln!("Actix server error: {}", e);
            }
        });
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .manage(share_map)
        .invoke_handler(tauri::generate_handler![
            encrypt_file,
            decrypt_file,
            get_local_ip
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
