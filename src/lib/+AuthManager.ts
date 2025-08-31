import { Store } from "@tauri-apps/plugin-store";
import { get, writable } from "svelte/store";

export const isAuthenticated = writable(false);

const STORE_KEY_SALT = 'auth.salt';
const STORE_KEY_HASH = 'auth.hash';

export let store: Store | null = null;

export async function initialiseStore(): Promise<void> {
    store = await Store.load('authentication.json');
}

async function getStoreValue(key: string): Promise<Uint8Array | null> {
    if (!store) throw new Error("Store not initialised");
    const result = await store.get(key);
    return result ? new Uint8Array(Object.values(result as object)) : null;
}

async function setStoreValue(key: string, value: Uint8Array): Promise<void> {
    if (!store) throw new Error("Store not initialised");
    await store.set(key, Array.from(value));
    await store.save();
}

export async function generateSalt(): Promise<Uint8Array> {
    const salt = crypto.getRandomValues(new Uint8Array(16));
    return salt;
}

export async function deriveKey(password: string, salt: Uint8Array): Promise<CryptoKey> {
    const enc = new TextEncoder();
    const baseKey = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveKey']);

    const pbkdf2Params: Pbkdf2Params = {
        name: 'PBKDF2',
        salt: salt as Uint8Array<ArrayBuffer>, // web crypto wants ArrayBuffer not SharedArrayBuffer
        iterations: 100000,
        hash: 'SHA-256'
    };

    return crypto.subtle.deriveKey(
        pbkdf2Params,
        baseKey,
        { name: 'AES-GCM', length: 256 },
        true,
        ['encrypt', 'decrypt']
    );
}

export async function hashKey(key: CryptoKey): Promise<ArrayBuffer> {
    const exported = await crypto.subtle.exportKey('raw', key);
    return crypto.subtle.digest('SHA-256', exported);
}

export async function setupPassword(password: string): Promise<void> {
    let salt = await getStoreValue(STORE_KEY_SALT);
    if (!salt) salt = await generateSalt();
    const key = await deriveKey(password, salt);
    const hash = await hashKey(key);
    await setStoreValue(STORE_KEY_SALT, salt);
    await setStoreValue(STORE_KEY_HASH, new Uint8Array(hash));
    isAuthenticated.set(true);
}

export async function verifyPassword(password: string): Promise<boolean> {
    const salt = await getStoreValue(STORE_KEY_SALT);
    if (!salt) return false;
    const key = await deriveKey(password, salt);
    const hash = await hashKey(key);
    const storedHash = await getStoreValue(STORE_KEY_HASH);
    if (!storedHash) return false;

    const hashView = new Uint8Array(hash);
    const storedView = new Uint8Array(storedHash);

    const match = hashView.byteLength === storedView.byteLength &&
        hashView.every((val, idx) => val === storedView[idx]);
    if (match) isAuthenticated.set(true);
    return match;
}