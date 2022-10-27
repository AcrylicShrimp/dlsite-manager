<script>
  import { invoke } from "@tauri-apps/api/tauri";
  import { onMount } from "svelte";

  onMount(async () => {
    invoke("show_window");
  });
</script>
