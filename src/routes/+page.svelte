<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";

  let title = "Hello Secure File Sharing";

  onMount(() => {
    // Will call tauri commands when DOM loads
  });
</script>

<main>
  <h1>{title}</h1>
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
