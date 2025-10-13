use actix_cors::Cors;
use actix_web::{http::header, web, App, HttpResponse, HttpServer, Result};
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
    salt: String,
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
            salt: data.salt.clone(),
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
    salt: String,
}

#[derive(serde::Serialize)]
struct EncryptedFileResult {
    encrypted: String,
    share_code: String,
    salt: Vec<u8>,
}

#[derive(serde::Serialize, Debug)]
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
    salt: Vec<u8>,
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
        salt: base64::engine::general_purpose::STANDARD.encode(&salt),
    };

    {
        let mut map = share_map.lock().unwrap();
        map.insert(share_code.clone(), share_data);
    }

    Ok(EncryptedFileResult {
        encrypted: encrypted_base64,
        share_code,
        salt,
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
                    .wrap(
                        Cors::default()
                            .allowed_origin("http://localhost:1420")
                            .allowed_origin("tauri://localhost")
                            .allowed_methods(vec!["GET", "POST"])
                            .allowed_headers(vec![header::AUTHORIZATION, header::ACCEPT])
                            .allowed_header(header::CONTENT_TYPE)
                            .max_age(3600)
                            .supports_credentials(),
                    )
                    .app_data(web::Data::new(share_map_for_server.clone()))
                    .route("/share/{code}", web::get().to(share_handler))
            })
            .bind(&addr)
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_file(content: &[u8]) -> (TempDir, String) {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test_file.txt");
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        (temp_dir, file_path.to_str().unwrap().to_string())
    }

    fn generate_test_key() -> Vec<u8> {
        vec![
            1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
            25, 26, 27, 28, 29, 30, 31, 32,
        ]
    }

    fn generate_test_salt() -> Vec<u8> {
        vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
    }

    async fn test_encrypt_file(
        path: String,
        key_bytes: Vec<u8>,
        salt: Vec<u8>,
        share_map: &ShareMap,
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
            salt: base64::engine::general_purpose::STANDARD.encode(&salt),
        };

        {
            let mut map = share_map.lock().unwrap();
            map.insert(share_code.clone(), share_data);
        }

        Ok(EncryptedFileResult {
            encrypted: encrypted_base64,
            share_code,
            salt,
        })
    }

    #[tokio::test]
    async fn test_encrypt_file_success() {
        let test_content = b"Hello, World! This is a test file.";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();

        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result = test_encrypt_file(file_path, key_bytes, salt.clone(), &share_map).await;

        assert!(result.is_ok());
        let encrypted_result = result.unwrap();

        assert!(!encrypted_result.encrypted.is_empty());

        assert!(encrypted_result.share_code.contains("-"));
        assert_eq!(encrypted_result.share_code.len(), 36);

        assert_eq!(encrypted_result.salt, salt);

        let map = share_map.lock().unwrap();
        assert!(map.contains_key(&encrypted_result.share_code));
    }

    #[tokio::test]
    async fn test_encrypt_file_invalid_path() {
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result = test_encrypt_file(
            "/nonexistent/file.txt".to_string(),
            key_bytes,
            salt,
            &share_map,
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_encrypt_file_invalid_key_length() {
        let test_content = b"Test content";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let invalid_key = vec![1, 2, 3];
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result = test_encrypt_file(file_path, invalid_key, salt, &share_map).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_file_success() {
        let test_content = b"Secret message for decryption test";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let encrypt_result = test_encrypt_file(file_path, key_bytes.clone(), salt, &share_map)
            .await
            .unwrap();

        let decrypt_result = decrypt_file(encrypt_result.encrypted, key_bytes).await;

        assert!(decrypt_result.is_ok());
        let decrypted = decrypt_result.unwrap();
        assert_eq!(decrypted.decrypted, test_content);
    }

    #[tokio::test]
    async fn test_decrypt_file_wrong_key() {
        let test_content = b"Secret data";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let wrong_key = vec![
            32, 31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11,
            10, 9, 8, 7, 6, 5, 4, 3, 2, 1,
        ];
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let encrypt_result = test_encrypt_file(file_path, key_bytes, salt, &share_map)
            .await
            .unwrap();

        let decrypt_result = decrypt_file(encrypt_result.encrypted, wrong_key).await;

        assert!(decrypt_result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_file_invalid_data() {
        let key_bytes = generate_test_key();
        let invalid_encrypted = "invalid_base64_data";

        let result = decrypt_file(invalid_encrypted.to_string(), key_bytes).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_decrypt_file_too_short() {
        let key_bytes = generate_test_key();
        let short_data = base64::engine::general_purpose::STANDARD.encode(&[1, 2, 3]);

        let result = decrypt_file(short_data, key_bytes).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid data"));
    }

    #[tokio::test]
    async fn test_share_data_stored_correctly() {
        let test_content = b"Share test";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result = test_encrypt_file(file_path.clone(), key_bytes, salt.clone(), &share_map)
            .await
            .unwrap();

        let map = share_map.lock().unwrap();
        let share_data = map.get(&result.share_code).unwrap();

        assert_eq!(share_data.encrypted, result.encrypted);
        assert_eq!(share_data.use_count, 0);
        assert_eq!(share_data.original_filename, "test_file.txt");

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(share_data.expiry > now);
        assert!(share_data.expiry <= now + 3600);
    }

    #[tokio::test]
    async fn encryption_produces_different_output() {
        let test_content = b"Same content";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result1 = test_encrypt_file(
            file_path.clone(),
            key_bytes.clone(),
            salt.clone(),
            &share_map,
        )
        .await
        .unwrap();

        let result2 = test_encrypt_file(file_path, key_bytes, salt, &share_map)
            .await
            .unwrap();

        assert_ne!(result1.encrypted, result2.encrypted);
        assert_ne!(result1.share_code, result2.share_code);
    }

    #[tokio::test]
    async fn test_encrypt_decrypt_roundtrip() {
        let test_contents = vec![
            b"Some random text".to_vec(),
            b"Text with some \nnewlines\nand\ttabs".to_vec(),
            b"Binary data: \x00\x01\x02\xFF".to_vec(),
            vec![0u8; 1000],                                    // Large binary data
            b"Unicode: \xE2\x9C\x93 \xF0\x9F\x98\x80".to_vec(), // UTF-8
        ];

        for test_content in test_contents {
            let (_temp_dir, file_path) = create_test_file(&test_content);
            let key_bytes = generate_test_key();
            let salt = generate_test_salt();
            let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

            let encrypt_result = test_encrypt_file(file_path, key_bytes.clone(), salt, &share_map)
                .await
                .unwrap();

            let decrypt_result = decrypt_file(encrypt_result.encrypted, key_bytes)
                .await
                .unwrap();

            assert_eq!(decrypt_result.decrypted, test_content);
        }
    }

    #[tokio::test]
    async fn test_share_code_format() {
        let test_content = b"Test";
        let (_temp_dir, file_path) = create_test_file(test_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let result = test_encrypt_file(file_path, key_bytes, salt, &share_map)
            .await
            .unwrap();

        // Verify UUID v4 format: xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx
        let parts: Vec<&str> = result.share_code.split('-').collect();
        assert_eq!(parts.len(), 5);
        assert_eq!(parts[0].len(), 8);
        assert_eq!(parts[1].len(), 4);
        assert_eq!(parts[2].len(), 4);
        assert_eq!(parts[3].len(), 4);
        assert_eq!(parts[4].len(), 12);
    }

    #[tokio::test]
    async fn test_large_file_encryption() {
        let large_content = vec![42u8; 150 * 1024 * 1024];
        let (_temp_dir, file_path) = create_test_file(&large_content);
        let key_bytes = generate_test_key();
        let salt = generate_test_salt();
        let share_map: ShareMap = Arc::new(Mutex::new(HashMap::new()));

        let encrypt_result =
            test_encrypt_file(file_path, key_bytes.clone(), salt, &share_map).await;

        assert!(encrypt_result.is_ok());

        let decrypt_result = decrypt_file(encrypt_result.unwrap().encrypted, key_bytes).await;

        assert!(decrypt_result.is_ok());
        assert_eq!(decrypt_result.unwrap().decrypted.len(), large_content.len());
    }
}
