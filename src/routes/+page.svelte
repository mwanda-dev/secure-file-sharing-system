<script lang="ts">
  import { onMount } from "svelte";
  import {
    setupPassword,
    verifyPassword,
    initialiseStore,
    store,
  } from "../lib/+AuthManager";
  import { isAuthenticated } from "../lib/+AuthManager";
  import { message } from "@tauri-apps/plugin-dialog";
  import { listen } from "@tauri-apps/api/event";

  let title = "Secure File Sharing System";

  let password = $state("");
  let status = $state("");
  let showLogin = $state(false);

  onMount(async () => {
    await initialiseStore();
    let salt = await store?.get("auth.salt");
    showLogin = !!salt;
  });

  async function handleSubmit() {
    try {
      if (showLogin) {
        const success = await verifyPassword(password);
        status = success ? "Logged in!" : "Wrong password";
        await message("Login successful!");
        if (success) {
          showLogin = false;
        } else {
          await message("Login failed: Wrong password");
          status = "Login failed: Wrong password";
        }
      } else {
        await setupPassword(password);
        status = "Password set!";
        await message("Setup successful!");
        showLogin = true;
      }
      password = "";
    } catch (error: unknown) {
      const errorMessage =
        // Teh error might not be of the type Error
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error) || "An unknown error has occured";
      status = "Error: " + errorMessage;
      await message("Error during auth: " + errorMessage);
    }
  }
</script>

<main>
  <h1>{title}</h1>
  {#if $isAuthenticated}
    <h2>Welcome to Secure Share</h2>
    <h3>You are now authenticated</h3>
  {:else}
    <h2>{showLogin ? "Login" : "Set Password"}</h2>
    <input type="password" bind:value={password} />
    <button onclick={handleSubmit}
      >{showLogin ? "Login" : "Set Password"}</button
    >
    <p>{status}</p>
  {/if}
</main>

<style>
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  main {
    text-align: center;
    padding: 1em;
  }

  h1 {
    color: black;
  }
</style>
