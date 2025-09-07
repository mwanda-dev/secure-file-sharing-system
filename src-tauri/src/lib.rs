use base64::Engine;

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
async fn encrypt_file(path: String, key_bytes: Vec<u8>) -> Result<EncryptedFileResult, String> {
    use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
    use ring::rand::{SecureRandom, SystemRandom};
    use std::fs::read;
    use uuid::Uuid;

    let data = read(path).map_err(|error| error.to_string())?;
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

    Ok(EncryptedFileResult {
        encrypted: encrypted_base64,
        share_code,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![encrypt_file, decrypt_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
