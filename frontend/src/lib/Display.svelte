<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { encodeMessage, MessageType, parseMessage } from './messages';
  import { fly, scale } from 'svelte/transition';

  const url = window.location.host;

  let enabled = $state(false);
  let playing = $state(false);
  let numNames = $state(0);
  let names = $state([] as string[]);
  let stillIn = $state([] as boolean[]);

  let socket: WebSocket;
  onMount(() => {
    socket = new WebSocket('/ws/display');
    socket.binaryType = 'arraybuffer';
    socket.addEventListener('open', () => {
      enabled = true;
      socket.send(
        encodeMessage({
          type: MessageType.Submitting,
          content: null,
        }),
      );
    });
    socket.addEventListener('message', (event) => {
      const message = parseMessage(event.data);
      switch (message.type) {
        case MessageType.NumNames:
          numNames = message.content;
          playing = false;
          names = [];
          stillIn = [];
          break;
        case MessageType.Names:
          names = message.content;
          stillIn = names.map(() => true);
          playing = true;
          break;
      }
    });
    socket.addEventListener('close', () => {
      enabled = false;
    });
  });

  onDestroy(() => {
    if (socket.readyState === WebSocket.OPEN) {
      socket.close();
    }
  });

  function buttonClicked() {
    if (playing) {
      socket.send(
        encodeMessage({ type: MessageType.Submitting, content: null }),
      );
    } else {
      socket.send(
        encodeMessage({ type: MessageType.NotSubmitting, content: null }),
      );
    }
  }
</script>

<div class="flex h-full flex-col">
  <header
    class="bg-surface-50-950/75 border-surface-100-900/30 w-full border p-8 text-center"
  >
    <div class="grid grid-cols-3 items-center">
      <div></div>
      <div class="flex flex-col gap-6">
        <h1 class="font-chewy text-6xl">The Name Game!</h1>
        <p class="justify-self-center text-3xl">Go to {url}</p>
      </div>
      <button
        class="btn preset-filled-primary-500 transition-colors-100 justify-self-end px-4 py-2 text-2xl"
        disabled={!enabled || numNames === 0}
        onclick={buttonClicked}
      >
        {playing ? 'Next round' : 'Show names'}
      </button>
    </div>
  </header>
  <main class="flex min-h-0 grow flex-col overflow-y-auto text-center">
    <div class="flex grow flex-col justify-center p-4">
      {#if playing}
        <ul class="flex flex-col gap-2 overflow-y-scroll text-3xl">
          {#each names as name, index (index)}
            <li in:fly|global={{ x: -200, opacity: 0, delay: index * 25 }}>
              <button
                class={[
                  'transition-colors-100 duration-50 hover:line-through',
                  !stillIn[index] && 'guessed',
                ]}
                onclick={() => (stillIn[index] = false)}
              >
                {name}
              </button>
            </li>
          {/each}
        </ul>
      {:else}
        <p class="text-3xl" in:scale>
          <span class="font-chewy text-6xl">{numNames}</span><br />
          names submitted
        </p>
      {/if}
    </div>
  </main>
</div>

<style>
  .guessed {
    text-decoration-line: line-through;
    color: var(--color-gray-500);
  }
</style>
