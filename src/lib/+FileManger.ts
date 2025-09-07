import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { writeFile } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";

let shareStore: Store | null = null;

async function getShareStore(): Promise<Store> {
    if (!shareStore) {
        shareStore = await Store.load('shares.json');
    }
    return shareStore;
}

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
    try {
        const store = await getShareStore();
        const expiry = Date.now() + 24 * 60 * 60 * 1000;
        const metadata = {
            encrypted,
            expiry,
            useCount: 0
        };

        await store.set(`share:${share_code}`, metadata);
        await store.save();
        await writeText(share_code);
    } catch (error) {
        console.error("Error in storing and sharing: ", error);
        throw error;
    }
}

export async function downloadAndDecrypt(share_code: string, keyBytes: Uint8Array): Promise<void> {
    try {
        const store = await getShareStore();
        const metadata = await store.get(`share:${share_code}`) as any;

        if (!metadata) {
            throw new Error('Invalid share code');
        }

        if (Date.now() > metadata.expiry) {
            await store.delete(`share:${share_code}`);
            await store.save();
            throw new Error('Share code has expired');
        }

        if (metadata.useCount > 0) {
            throw new Error('Share code has already been used');
        }

        const result = await invoke<{ decrypted: number[] }>('decrypt_file', {
            encryptedBase64: metadata.encrypted,
            keyBytes: Array.from(keyBytes)
        });

        const decryptedBytes = new Uint8Array(result.decrypted);

        const savePath = await save({
            defaultPath: 'decrypted_file'
        });

        if (!savePath) {
            throw new Error('Save operation was cancelled');
        }

        await writeFile(savePath, decryptedBytes);

        await store.delete(`share:${share_code}`);
        await store.save();

    } catch (error) {
        console.error("Error in downloading and decrypt: ", error);
        throw error;
    }
}