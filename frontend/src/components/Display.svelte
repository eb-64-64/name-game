<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { MessageType } from '../lib/messages';
  import { fly, scale } from 'svelte/transition';
  import { GameState } from '../lib/state';
  import { ReconnectingSocket } from '../lib/reconnecting-socket';
  import DisconnectionToast from './DisconnectionToast.svelte';

  const url = window.location.host;

  let connected = $state(true);
  let gameState:
    | { state: GameState.Submitting; numNames: number }
    | { state: GameState.Playing; names: string[]; guesses: boolean[] } =
    $state({
      state: GameState.Submitting,
      numNames: 0,
    });

  let socket: ReconnectingSocket;
  onMount(() => {
    socket = new ReconnectingSocket('/ws/display');
    socket.onOpen = () => {
      connected = true;
    };
    socket.onMessage = (message) => {
      switch (message.type) {
        case MessageType.NumNames:
          gameState = {
            state: GameState.Submitting,
            numNames: message.content,
          };
          break;
        case MessageType.Names:
          gameState = {
            state: GameState.Playing,
            names: message.content[0],
            guesses: message.content[1],
          };
          break;
        case MessageType.NameGuessed:
          if (gameState.state === GameState.Playing) {
            gameState.guesses[message.content] = true;
          }
          break;
      }
    };
    socket.onClose = () => {
      connected = false;
    };
  });

  onDestroy(() => {
    socket.close();
  });

  function buttonClicked() {
    if (gameState.state === GameState.Playing) {
      socket.send({ type: MessageType.StateSubmitting, content: null });
    } else if (gameState.state === GameState.Submitting) {
      socket.send({ type: MessageType.StatePlaying, content: null });
    }
  }

  function nameClicked(index: number): () => void {
    return () => {
      socket.send({ type: MessageType.MakeGuess, content: index });
    };
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
        disabled={!connected ||
          (gameState.state === GameState.Submitting &&
            gameState.numNames === 0)}
        onclick={buttonClicked}
      >
        {#if gameState.state === GameState.Submitting}
          Show names
        {:else}
          Next round
        {/if}
      </button>
    </div>
  </header>
  <main class="flex min-h-0 grow flex-col overflow-y-auto text-center">
    <div class="flex grow flex-col justify-center p-4">
      {#if gameState.state === GameState.Submitting}
        <p class="text-3xl" in:scale>
          <span class="font-chewy text-6xl">{gameState.numNames}</span><br />
          names submitted
        </p>
      {:else}
        <ul class="flex flex-col gap-2 overflow-y-scroll text-3xl">
          {#each gameState.names as name, index (index)}
            <li in:fly|global={{ x: -200, opacity: 0, delay: index * 25 }}>
              <button
                class={[
                  'transition-colors-100 duration-50 hover:line-through',
                  gameState.guesses[index] && 'guessed',
                ]}
                onclick={nameClicked(index)}
              >
                {name}
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </main>
</div>

<DisconnectionToast {connected} />

<style>
  .guessed {
    text-decoration-line: line-through;
    color: var(--color-gray-500);
  }
</style>
