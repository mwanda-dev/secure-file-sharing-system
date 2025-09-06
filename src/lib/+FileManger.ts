import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

export async function uploadAndEncrypt(keyBytes: Uint8Array): Promise<{ share_code: string; encrypted: string }> {
    try {
        const selected = await open({ multiple: false, directory: false }) as string | { path: string } | null;
        if (!selected) throw new Error("No file selected");

        const path = typeof selected === 'string' ? selected : (selected as { path: string }).path;
        console.log('Encrypting file at path: ', path);
        console.log('Key bytes length: ', keyBytes.length);

        const result = await invoke<{ encrypted: string; share_code: string }>('encrypt_file', { path, keyBytes: Array.from(keyBytes) });

        console.log('Encryption result: ', result);


        // if (!result.encrypted) {
        //     throw new Error("No encrypted data returned");
        // }

        // if (!result.shareCode) {
        //     throw new Error("No share code returned");
        // }
        return {
            encrypted: result.encrypted,
            share_code: result.share_code
        };
    } catch (error) {
        console.error("Error in uploading and encrypt: ", error);
        throw error;
    }
}