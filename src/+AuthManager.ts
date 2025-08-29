import { invoke } from "@tauri-apps/api/core";
import { get, writable } from "svelte/store";

const isAuthenticated = writable(false);