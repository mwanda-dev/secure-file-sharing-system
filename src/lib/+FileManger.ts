import { open, save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { writeText } from "@tauri-apps/plugin-clipboard-manager";
import { writeFile } from "@tauri-apps/plugin-fs";
import { Store } from "@tauri-apps/plugin-store";
import { basename } from "@tauri-apps/api/path";
import Swal from "sweetalert2";

let shareStore: Store | null = null;

interface ShareMetaData {
    encrypted: string;
    expiry: number;
    useCount: number;
    originalFileName: string;
}

async function getShareStore(): Promise<Store> {
    if (!shareStore) {
        shareStore = await Store.load('shares.json');
    }
    return shareStore;
}

export async function uploadAndEncrypt(keyBytes: Uint8Array): Promise<{ share_code: string; encrypted: string, originalFileName: string }> {
    try {
        const selected = await open({ multiple: false, directory: false, title: "Select file to send" }) as string | { path: string } | null;
        if (!selected) throw new Error("No file selected");

        const path = typeof selected === 'string' ? selected : (selected as { path: string }).path;
        console.log('Encrypting file at path: ', path);
        console.log('Key bytes length: ', keyBytes.length);

        const originalFileName = await basename(path);

        const result = await invoke<{ encrypted: string; share_code: string }>('encrypt_file', { path, keyBytes: Array.from(keyBytes) });

        console.log('Encryption result: ', result);

        return {
            encrypted: result.encrypted,
            share_code: result.share_code,
            originalFileName
        };
    } catch (error) {
        console.error("Error in uploading and encrypt: ", error);
        throw error;
    }
}

export async function storeAndShare(encrypted: string, share_code: string, originalFileName: string): Promise<void> {
    try {
        const store = await getShareStore();
        const expiry = Date.now() + 24 * 60 * 60 * 1000;
        const metadata = {
            encrypted,
            expiry,
            useCount: 0,
            originalFileName: originalFileName
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
        if (!/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(share_code)) {
            throw new Error('Invalid share code format');
        }

        let metadata: ShareMetaData | null = null;
        let fromNetwork = false;

        try {
            const ip = await promptForIP();
            const url = `http://${ip}:8080/share/${share_code}`;
            const response = await fetch(url);
            if (!response.ok) throw new Error(`HTTP ${response.status}`);
            const remoteData = await response.json();
            metadata = {
                encrypted: remoteData.encrypted,
                expiry: Date.now() + 3600 * 1000,
                useCount: 0,
                originalFileName: remoteData.original_filename
            };
            fromNetwork = true;
        } catch (networkError) {
            console.log("Network fetch failed, falling back to local store: ", networkError);
            const store = await getShareStore();
            const raw = await store.get(`share:${share_code}`);
            if (!raw || typeof raw !== 'object') {
                throw new Error('Invalid or missing share metadata (local)');
            }
            const localMeta = raw as Partial<ShareMetaData>;
            if (!localMeta.encrypted || typeof localMeta.expiry !== 'number' || typeof localMeta.useCount !== 'number') {
                throw new Error('Corrupted share metadata (local)');
            }
            metadata = localMeta as ShareMetaData;
            fromNetwork = false;
        }

        if (!metadata) throw new Error('No metadata found');

        if (Date.now() > metadata.expiry) {
            if (!fromNetwork) {
                const store = await getShareStore();
                await store.delete(`share:${share_code}`);
                await store.save();
            }
            throw new Error('Share code has expired');
        }

        if (metadata.useCount >= 1 && !fromNetwork) {
            throw new Error('Share code has already been used');
        }

        if (!fromNetwork) {
            const store = await getShareStore();
            metadata.useCount += 1;
            await store.set(`share:${share_code}`, metadata);
            await store.save();
        }

        const result = await invoke<{ decrypted: number[] }>('decrypt_file', {
            encryptedBase64: metadata.encrypted,
            keyBytes: Array.from(keyBytes)
        });

        const decryptedBytes = new Uint8Array(result.decrypted);
        const defaultFileName = metadata.originalFileName || "decrypted_file";

        const savePath = await save({
            defaultPath: defaultFileName,
            title: "Save Decrypted File"
        });

        if (!savePath) {
            throw new Error('Save operation was cancelled');
        }

        await writeFile(savePath, decryptedBytes);

        if (!fromNetwork) {
            const store = await getShareStore();
            await store.delete(`share:${share_code}`);
            await store.save();
        }

    } catch (error) {
        console.error("Error in downloading and decrypting: ", error);
        throw error;
    }

}

async function promptForIP(): Promise<string> {
    const { value: ip } = await Swal.fire({
        title: 'Enter Sender IP address',
        input: 'text',
        inputLabel: 'IP Address',
        inputPlaceholder: '192.168.0.1',
        // inputValue: '127.0.0.1',
        inputValidator: (value) => {
            if (!value) return 'IP is required';
            const ipRegex = /^(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)$/;
            if (!ipRegex.test(value)) return 'Invalid IP format';
            return null;
        }
    });

    if (!ip) throw new Error('IP address is required');
    return ip;
}