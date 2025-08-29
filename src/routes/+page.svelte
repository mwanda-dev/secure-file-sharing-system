<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let message = "Hello Secure File Sharing";

  onMount(() => {
    // Will call tauri commands when DOM loads
  });
</script>

<main>
  <h1>{message}</h1>
  <button on:click={() => alert("Test")}>Test Button</button>
</main>

<style>
  * {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
  }

  main {
    text-align: center;
  }
</style>
