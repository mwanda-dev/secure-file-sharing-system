<script lang="ts">
  import { onMount } from "svelte";
  import {
    setupPassword,
    verifyPassword,
    initialiseStore,
    store,
  } from "../lib/+AuthManager";
  import { isAuthenticated } from "../lib/+AuthManager";
  import { listen } from "@tauri-apps/api/event";

  let title = "Hello Secure File Sharing";

  let password = $state("");
  let message = $state("");
  let showLogin = $state(false);

  onMount(async () => {
    await initialiseStore();
    let salt = await store?.get("auth.json");
    showLogin = !!salt;
  });

  async function handleSubmit() {
    try {
      if (showLogin) {
        const success = await verifyPassword(password);
        message = success ? "Logged in!" : "Wrong password";
        if (success) {
          alert("Login successful!");
          showLogin = false;
        } else {
          alert("Login failed: Wrong password");
        }
      } else {
        await setupPassword(password);
        message = "Password set!";
        alert("Setup successful!");
        showLogin = true;
      }
      password = "";
    } catch (error: unknown) {
      const errorMessage =
        error instanceof Error ? error.message : "An unkown error has occured";
      message = "Error: " + errorMessage;
      alert("Error during auth: " + errorMessage);
    }
  }
</script>

<main>
  <h1>{title}</h1>
  {#if $isAuthenticated}
    <h2>Greetings from Secure Share - You are authenticated</h2>
  {:else}
    <h2>{showLogin ? "Login" : "Set Password"}</h2>
    <input type="password" bind:value={password} />
    <button onclick={handleSubmit}
      >{showLogin ? "Login" : "Set Password"}</button
    >
    <p>{message}</p>
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
