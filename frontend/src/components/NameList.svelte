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

  const nameClasses = (index: number) => [
    'transition-colors-100 duration-50',
    guesses[index] && 'guessed',
  ];
</script>

<ul
  class={[
    'mx-auto grid grid-cols-1 place-items-center gap-x-32 gap-y-8 text-2xl',
    names.length > 8 && 'md:grid-cols-2',
  ]}
>
  {#each names as name, index (index)}
    <li
      in:fly|global={{ x: -200, opacity: 0, delay: index * 25 }}
      class="leading-none"
    >
      {#if clickability.clickable}
        <button
          class={[...nameClasses(index), 'hover:line-through']}
          onclick={() => clickability.onClick(index)}
        >
          {name}
        </button>
      {:else}
        <span class={nameClasses(index)}>
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
