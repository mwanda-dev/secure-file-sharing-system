#[tauri::command]
async fn encrypt_file(path: String, key_bytes: Vec<u8>) -> Result<(Vec<u8>, String), String> {
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

    let share_code = Uuid::new_v4().to_string();

    Ok((in_out, share_code))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![encrypt_file])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
