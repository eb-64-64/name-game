<script lang="ts">
  import { fly } from 'svelte/transition';

  let {
    names,
    guesses,
    clickability = { clickable: false },
  }: {
    names: string[];
    guesses: boolean[];
    clickability:
      | { clickable: false }
      | { clickable: true; onClick: (index: number) => void };
  } = $props();
</script>

<ul class="flex flex-col gap-2 overflow-y-scroll text-3xl">
  {#each names as name, index (index)}
    <li in:fly|global={{ x: -200, opacity: 0, delay: index * 25 }}>
      {#if clickability.clickable}
        <button
          class="transition-colors-100 duration-50 hover:line-through"
          class:guessed={guesses[index]}
          onclick={() => clickability.onClick(index)}
        >
          {name}
        </button>
      {:else}
        <span
          class="transition-colors-100 duration-50"
          class:guessed={guesses[index]}
        >
          {name}
        </span>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .guessed {
    text-decoration-line: line-through;
    color: var(--color-gray-500);
  }
</style>
