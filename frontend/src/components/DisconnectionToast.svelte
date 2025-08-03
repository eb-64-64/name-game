<script lang="ts">
  import { TriangleAlert } from '@lucide/svelte';
  import { fly } from 'svelte/transition';

  let { connected }: { connected: boolean } = $props();
</script>

<div
  role="status"
  aria-live="polite"
  class="pointer-events-none fixed bottom-0 z-50 flex w-full justify-center"
>
  {#if !connected}
    <div
      transition:fly={{ y: 88, opacity: 100 }}
      class="bg-error-400-600 items-middle pointer-events-auto mb-8 flex place-items-center gap-3 rounded-lg px-6 py-4"
    >
      <TriangleAlert /><span>Disconnected</span>
    </div>
  {/if}
</div>
