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

  :root {
    --primary-blue: #4a90e2;
    --light-blue: #e8f2ff;
    --dark-blue: #2c5282;
    --soft-white: #fafbfc;
    --pure-white: #ffffff;
    --light-grey: #f5f7fa;
    --medium-grey: #e1e4e8;
    --dark-grey: #6b7280;
    --text-dark: #1f2937;
    --success-green: #10b981;
    --error-red: #ef4444;
    --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
    --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.07);
    --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.1);
  }

  :global(body) {
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
      "Helvetica Neue", Arial, sans-serif;
    background: var(--soft-white);
    color: var(--text-dark);
    line-height: 1.6;
  }

  main {
    min-height: 100vh;
    position: relative;
  }

  .bg-gradient {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background: linear-gradient(
      135deg,
      var(--light-blue) 0%,
      var(--soft-white) 50%,
      var(--light-grey) 100%
    );
    z-index: -1;
  }

  .header {
    background: var(--pure-white);
    border-bottom: 1px solid var(--medium-grey);
    padding: 1rem 2rem;
    display: flex;
    justify-content: space-between;
    align-items: center;
    box-shadow: var(--shadow-sm);
  }

  .logo {
    display: flex;
    align-items: center;
    gap: 0.75rem;
  }

  .logo-icon {
    font-size: 1.5rem;
  }

  .logo h1 {
    font-size: 1.5rem;
    font-weight: 600;
    color: var(--text-dark);
  }

  .ip-display {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    background: var(--light-grey);
    padding: 0.5rem 1rem;
    border-radius: 8px;
  }

  .ip-label {
    color: var(--dark-grey);
    font-size: 0.875rem;
  }

  .ip-value {
    font-family: "Courier New", monospace;
    color: var(--text-dark);
    font-weight: 500;
  }

  .copy-btn {
    background: transparent;
    border: none;
    cursor: pointer;
    font-size: 1rem;
    padding: 0.25rem;
    transition: transform 0.2s;
  }

  .copy-btn:hover {
    transform: scale(1.1);
  }

  .content {
    max-width: 1200px;
    margin: 0 auto;
    padding: 2rem;
  }

  .dashboard {
    animation: fadeIn 0.5s ease-in;
  }

  .welcome-section {
    text-align: center;
    margin-bottom: 3rem;
  }

  .welcome-section h2 {
    font-size: 2rem;
    color: var(--text-dark);
    margin-bottom: 0.5rem;
    font-weight: 600;
  }

  .welcome-section p {
    color: var(--dark-grey);
    font-size: 1.125rem;
  }

  .action-cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
    gap: 2rem;
    margin-bottom: 2rem;
  }

  .card {
    background: var(--pure-white);
    border-radius: 12px;
    padding: 2rem;
    box-shadow: var(--shadow-md);
    transition:
      transform 0.3s,
      box-shadow 0.3s;
  }

  .card:hover {
    transform: translateY(-4px);
    box-shadow: var(--shadow-lg);
  }

  .card-icon {
    font-size: 3rem;
    margin-bottom: 1rem;
  }

  .card h3 {
    font-size: 1.25rem;
    color: var(--text-dark);
    margin-bottom: 0.75rem;
    font-weight: 600;
  }

  .card p {
    color: var(--dark-grey);
    margin-bottom: 1.5rem;
    font-size: 0.95rem;
  }

  .input-group {
    display: flex;
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .code-input,
  input[type="password"],
  input[type="text"] {
    flex: 1;
    padding: 0.75rem 1rem;
    border: 2px solid var(--medium-grey);
    border-radius: 8px;
    font-size: 1rem;
    transition: border-color 0.3s;
    background: var(--soft-white);
  }

  .code-input:focus,
  input[type="password"]:focus,
  input[type="text"]:focus {
    outline: none;
    border-color: var(--primary-blue);
    background: var(--pure-white);
  }

  .primary-btn,
  .auth-btn {
    background: var(--primary-blue);
    color: white;
    border: none;
    padding: 0.75rem 1.5rem;
    border-radius: 8px;
    font-size: 1rem;
    font-weight: 500;
    cursor: pointer;
    transition:
      background-color 0.3s,
      transform 0.1s;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: 0.5rem;
  }

  .primary-btn:hover:not(:disabled),
  .auth-btn:hover {
    background: var(--dark-blue);
    transform: translateY(-1px);
  }

  .primary-btn:disabled {
    background: var(--medium-grey);
    cursor: not-allowed;
    opacity: 0.7;
  }

  .auth-btn {
    width: 100%;
    margin-top: 1rem;
  }

  .share-result {
    margin-top: 2rem;
    animation: slideUp 0.4s ease-out;
  }

  .result-card {
    background: var(--pure-white);
    border-radius: 12px;
    padding: 2rem;
    box-shadow: var(--shadow-md);
    border: 2px solid var(--success-green);
  }

  .result-card h4 {
    color: var(--success-green);
    font-size: 1.25rem;
    margin-bottom: 1.5rem;
  }

  .share-code-display {
    margin-bottom: 1rem;
  }

  .code-label {
    display: block;
    color: var(--dark-grey);
    font-size: 0.875rem;
    margin-bottom: 0.5rem;
  }

  .code-container {
    display: flex;
    gap: 1rem;
    align-items: center;
    background: var(--light-grey);
    padding: 1rem;
    border-radius: 8px;
  }

  .share-code {
    flex: 1;
    font-family: "Courier New", monospace;
    font-size: 1.125rem;
    color: var(--text-dark);
    word-break: break-all;
  }

  .copy-btn-large {
    background: var(--primary-blue);
    color: white;
    border: none;
    padding: 0.5rem 1rem;
    border-radius: 6px;
    cursor: pointer;
    font-size: 0.875rem;
    font-weight: 500;
    transition: background-color 0.3s;
    white-space: nowrap;
  }

  .copy-btn-large:hover {
    background: var(--dark-blue);
  }

  .share-instructions {
    color: var(--dark-grey);
    font-size: 0.875rem;
    font-style: italic;
  }

  .auth-container {
    display: flex;
    justify-content: center;
    align-items: center;
    min-height: calc(100vh - 80px);
    padding: 2rem;
  }

  .auth-card {
    background: var(--pure-white);
    border-radius: 16px;
    padding: 3rem 2rem;
    box-shadow: var(--shadow-lg);
    max-width: 400px;
    width: 100%;
  }

  .auth-icon {
    font-size: 4rem;
    margin-bottom: 1.5rem;
  }

  .auth-card h2 {
    font-size: 1.75rem;
    color: var(--text-dark);
    margin-bottom: 0.75rem;
    font-weight: 600;
  }

  .auth-card p {
    color: var(--dark-grey);
    margin-bottom: 2rem;
  }

  .input-container {
    margin-bottom: 1rem;
  }

  .status-message {
    margin-top: 1rem;
    padding: 0.75rem;
    border-radius: 6px;
    font-size: 0.875rem;
    background: var(--light-blue);
    color: var(--dark-blue);
  }

  .status-message.error {
    background: #fee2e2;
    color: var(--error-red);
  }

  .spinner {
    width: 16px;
    height: 16px;
    border: 2px solid rgba(255, 255, 255, 0.3);
    border-top-color: white;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
</style>
