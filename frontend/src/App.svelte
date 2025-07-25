<script lang="ts">
  import { Router, type RouteConfig } from '@mateothegreat/svelte5-router';
  import Player from './components/Player.svelte';
  import Display from './components/Display.svelte';

  const routes: RouteConfig[] = [
    {
      component: Player,
    },
    {
      path: 'display',
      component: Display,
    },
  ];
</script>

<Router {routes} />
