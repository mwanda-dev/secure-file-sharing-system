import { writable } from "svelte/store";

export const isAuthenticated = writable(true);

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

export async function exportDerivedKey(password: string, salt: Uint8Array): Promise<Uint8Array> {
    const key = await deriveKey(password, salt);

    const exported = await crypto.subtle.exportKey('raw', key);
    return new Uint8Array(exported);
}