<script lang="ts">
  import { Router, type RouteConfig } from "@mateothegreat/svelte5-router";
    import Player from "./lib/Player.svelte";
    import Display from "./lib/Display.svelte";

  const routes: RouteConfig[] = [
    {
      component: Player,
    },
    {
      path: "display",
      component: Display,
    },
  ];
</script>

<Router {routes} />
