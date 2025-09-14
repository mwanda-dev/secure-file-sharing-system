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
  import { getLocalIp } from "$lib/+NetworkUtils";
  import { writeText } from "@tauri-apps/plugin-clipboard-manager";

  let title = "Secure Share";

  let password = $state("");
  let status = $state("");
  let showLogin = $state(false);

  let shareCode = $state("");
  let downloadCode = $state("");
  let isUploading = $state(false);
  let isDownloading = $state(false);

  let localIp = $state<string | null>(null);

  onMount(async () => {
    await initialiseStore();
    let salt = await store?.get("auth.salt");
    showLogin = !!salt;
    localIp = await getLocalIp();
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
            : JSON.stringify(error) || "An unknown error has occurred";
      status = "Error: " + errorMessage;
      await message("Error during auth: " + errorMessage);
    }
  }

  async function handleUpload() {
    try {
      isUploading = true;
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
      await storeAndShare(
        result.encrypted,
        result.share_code,
        result.originalFileName,
      );

      console.log(result);
      shareCode = result.share_code;
      await message(`File encrypted! Share code: ${shareCode}`);
    } catch (error) {
      const errorMessage =
        error instanceof Error
          ? error.message
          : typeof error === "string"
            ? error
            : JSON.stringify(error) || "An unknown error has occurred";
      status = "Error: " + errorMessage;
      await message("Error: " + errorMessage);
    } finally {
      isUploading = false;
    }
  }

  async function handleDownload() {
    try {
      isDownloading = true;
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
      downloadCode = "";
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
    } finally {
      isDownloading = false;
    }
  }

  async function copyToClipBoard(text: string) {
    await writeText(text);
  }

  function preventDefault(handler: { (): Promise<void>; (arg0: any): void }) {
    return (e: { preventDefault: () => void }) => {
      e.preventDefault();
      handler(e);
    };
  }
</script>

<main>
  <div class="bg-gradient"></div>

  <!-- Header -->
  <header class="header">
    <div class="logo">
      <div class="logo-icon">🔒</div>
      <h1>{title}</h1>
    </div>
    {#if localIp}
      <div class="ip-display">
        <span class="ip-label">Your IP:</span>
        <span class="ip-value">{localIp}</span>
        <button class="copy-btn" onclick={() => copyToClipBoard(localIp || "")}>
          📋
        </button>
      </div>
    {/if}
  </header>

  <!-- Main Content -->
  <div class="content">
    {#if $isAuthenticated}
      <div class="dashboard">
        <div class="welcome-section">
          <h2>Welcome to SecureShare</h2>
          <p>
            Encrypt, share, and decrypt files securely with end-to-end
            encryption
          </p>
        </div>

        <div class="action-cards">
          <!-- Upload Card -->
          <div class="card upload-card">
            <div class="card-icon">📤</div>
            <h3>Share a File</h3>
            <p>Select a file to encrypt and generate a secure share code</p>
            <button
              class="primary-btn"
              onclick={handleUpload}
              disabled={isUploading}
            >
              {#if isUploading}
                <div class="spinner"></div>
                Encrypting...
              {:else}
                Upload & Encrypt
              {/if}
            </button>
          </div>

          <!-- Download Card -->
          <div class="card download-card">
            <div class="card-icon">📥</div>
            <h3>Receive a File</h3>
            <p>Enter a share code to decrypt and download a file</p>
            <div class="input-group">
              <input
                type="text"
                bind:value={downloadCode}
                placeholder="Enter share code"
                class="code-input"
              />
              <button
                class="primary-btn"
                onclick={handleDownload}
                disabled={isDownloading || !downloadCode.trim()}
              >
                {#if isDownloading}
                  <div class="spinner"></div>
                  Decrypting...
                {:else}
                  Download
                {/if}
              </button>
            </div>
          </div>
        </div>

        {#if shareCode}
          <div class="share-result">
            <div class="result-card">
              <h4>✅ File Encrypted Successfully!</h4>
              <div class="share-code-display">
                <span class="code-label">Share Code:</span>
                <div class="code-container">
                  <code class="share-code">{shareCode}</code>
                  <button
                    class="copy-btn-large"
                    onclick={() => copyToClipBoard(shareCode)}
                    title="Copy to clipboard"
                  >
                    📋 Copy
                  </button>
                </div>
              </div>
              <p class="share-instructions">
                Share this code with the recipient. It will expire in 1 day and
                can only be used once.
              </p>
            </div>
          </div>
        {/if}
      </div>
    {:else}
      <div class="auth-container">
        <div class="auth-card">
          <div class="auth-icon">🛡️</div>
          <h2>{showLogin ? "Welcome Back" : "Setup Security"}</h2>
          <p>
            {showLogin
              ? "Enter your password to access SecureShare"
              : "Create a password to secure your file sharing"}
          </p>

          <form onsubmit={preventDefault(handleSubmit)}>
            <div class="input-container">
              <input
                type="password"
                bind:value={password}
                maxlength="64"
                placeholder="Enter password"
                required
              />
            </div>
            <button type="submit" class="auth-btn">
              {showLogin ? "🔓 Login" : "🔐 Set Password"}
            </button>
          </form>

          {#if status}
            <div class="status-message" class:error={status.includes("Error")}>
              {status}
            </div>
          {/if}
        </div>
      </div>
    {/if}
  </div>
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
