// import { window } from "@tauri-apps/api";
import { invoke } from "@tauri-apps/api/core";

export async function getLocalIp(): Promise<string> {
    try {
        const ip = await invoke<string>('get_local_ip');
        return ip;
    } catch {
        return "127.0.0.1";
    }
}