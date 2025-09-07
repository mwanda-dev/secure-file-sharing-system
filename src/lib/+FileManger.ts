import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { store } from "./+AuthManager";

export async function uploadAndEncrypt(keyBytes: Uint8Array): Promise<{ share_code: string; encrypted: string }> {
    try {
        const selected = await open({ multiple: false, directory: false }) as string | { path: string } | null;
        if (!selected) throw new Error("No file selected");

        const path = typeof selected === 'string' ? selected : (selected as { path: string }).path;
        console.log('Encrypting file at path: ', path);
        console.log('Key bytes length: ', keyBytes.length);

        const result = await invoke<{ encrypted: string; share_code: string }>('encrypt_file', { path, keyBytes: Array.from(keyBytes) });

        console.log('Encryption result: ', result);

        return {
            encrypted: result.encrypted,
            share_code: result.share_code
        };
    } catch (error) {
        console.error("Error in uploading and encrypt: ", error);
        throw error;
    }
}

export async function storeAndShare(encrypted: string, share_code: string): Promise<void> {
    const expiry = Date.now() + 24 * 60 * 60 * 1000;  // 24h
    const metadata = { encrypted, expiry, useCount: 0 };
    await store.set(`share:${share_code}`, metadata);
    await store.save();
    await write({ text: share_code });
}

export async function downloadAndDecrypt(share_code: string, keyBytes: Uint8Array): Promise<void> {
    const metadata = await store.get(`share:${share_code}`);
    if (!metadata) throw new Error('Invalid code');
    if (Date.now() > metadata.expiry || metadata.useCount > 0) throw new Error('Expired or used');

    const { decrypted } = await invoke('decrypt_file', { encryptedBase64: metadata.encrypted, keyBytes: Array.from(keyBytes) });

    const savePath = await save({ defaultPath: 'decrypted.file' });
    if (!savePath) throw new Error('Save cancelled');
    await invoke('write_file', { path: savePath, contents: decrypted });  // Assume fs write command or use fs plugin

    metadata.useCount += 1;
    if (metadata.useCount > 0) await store.delete(`share:${share_code}`);
    else await store.set(`share:${share_code}`, metadata);
    await store.save();
}