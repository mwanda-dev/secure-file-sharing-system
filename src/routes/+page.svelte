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
  import { uploadAndEncrypt } from "$lib/+FileManger";
  import { exportDerivedKey, getStoreValue } from "../lib/+AuthManager";
  import Swal from "sweetalert2";
  import { storeAndShare, downloadAndDecrypt } from "$lib/+FileManger";

  let title = "Secure File Sharing System";

  let password = $state("");
  let status = $state("");
  let showLogin = $state(false);

  let shareCode = $state("");
  let downloadCode = $state("");

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
        if (success) {
          showLogin = false;
          await message("Login successful!");
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
        // The error might not be of the type Error
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error) || "An unknown error has occured";
      status = "Error: " + errorMessage;
      await message("Error during auth: " + errorMessage);
    }
  }

  async function handleUpload() {
    try {
      const salt = await getStoreValue("auth.salt");
      if (!salt) throw new Error("Not authenticated");
      // No prompt in plugin-dialog
      const { value: password } = await Swal.fire({
        title: "Enter your password",
        input: "password",
        inputLabel: "Password",
        inputPlaceholder: "Enter your password",
        inputAttributes: {
          maxlength: "64",
          autocapitalize: "off",
          autocorrect: "off",
        },
      });
      if (!password) throw new Error("Password is required for key export");
      const keyBytes = await exportDerivedKey(password, salt);
      const result = await uploadAndEncrypt(keyBytes);
      await storeAndShare(result.encrypted, result.share_code);
      console.log(result);
      shareCode = result.share_code;
      await message(`File encrypted! Share code: ${shareCode}`);
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error) || "An unknown error has occured";
      status = "Error: " + errorMessage;
      await message("Error: " + errorMessage);
    }
  }

  async function handleDownload() {
    try {
      const salt = await getStoreValue("auth.salt");
      if (!salt) throw new Error("Not authenticated - salt not found");

      const { value: password } = await Swal.fire({
        title: "Enter your password",
        input: "password",
        inputLabel: "Password",
        inputPlaceholder: "Enter your password",
        inputAttributes: {
          maxlength: "64",
          autocapitalize: "off",
          autocorrect: "off",
        },
      });

      if (!password) throw new Error("Password is required for key export");

      const keyBytes = await exportDerivedKey(password, salt);
      await downloadAndDecrypt(downloadCode, keyBytes);
      await message("File has been downloaded and decrypted.");
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error) || "An unknown error has occurred";
      status = "Error: " + errorMessage;
      await message("Error: " + errorMessage);
    }
  }
</script>

<main>
  <h1>{title}</h1>
  {#if $isAuthenticated}
    <h2>Welcome to Secure Share</h2>
    <button onclick={handleUpload}>Upload and Encrypt</button>
    <input
      type="text"
      bind:value={downloadCode}
      placeholder="Enter share code"
    />
    <button onclick={handleDownload}>Download and Decrypt</button>
    {#if shareCode}
      <p>Share Code: {shareCode}</p>
    {/if}
  {:else}
    <h2>{showLogin ? "Login" : "Set Password"}</h2>
    <input type="password" bind:value={password} maxlength="64" />
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
