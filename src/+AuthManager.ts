import { invoke } from "@tauri-apps/api/core";
import { get, writable } from "svelte/store";

const isAuthenticated = writable(false);

const KEY_SALT = 'auth.salt';
const KEY_HASH = 'auth.hash';

async function get_store_value_key(key: string): Promise<Uint8Array | null> {
    return invoke('plugin:store|get', { key }).then((val: any) => val ? new Uint8Array(Object.values(val)) : null);
}

async function set_store_value(key: string, value: Uint8Array): Promise<void> {
    await invoke('plugin:store|set', { key, value: Array.from(value) })
}

export async function generate_salt(): Promise<Uint8Array> {
    const salt = crypto.getRandomValues(new Uint8Array(16));
    return salt;
}

