import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";

export async function uploadAndEncrypt(keyBytes: Uint8Array): Promise<{ shareCode: string }> {
    const selected = await open({ multiple: false, directory: false }) as string | { path: string } | null;
    if (!selected) throw new Error("No file selected");

    const path = typeof selected === 'string' ? selected : (selected as { path: string }).path;
    const { encrypted, shareCode } = await invoke<{ encrypted: string; shareCode: string }>('encrypt_file', { path, keyBytes: Array.from(keyBytes) });

    return { shareCode };
}