<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { decodeMessage, encodeMessage, MessageType } from '../lib/messages';
  import { fly, scale } from 'svelte/transition';
  import { GameState } from '../lib/state';
  import Spinner from './Spinner.svelte';

  const url = window.location.host;

  let state:
    | { gameState: GameState.Disconnected }
    | { gameState: GameState.Submitting; numNames: number }
    | { gameState: GameState.Playing; names: string[]; guesses: boolean[] } =
    $state({
      gameState: GameState.Disconnected,
    });

  let socket: WebSocket;
  onMount(() => {
    socket = new WebSocket('/ws/display');
    socket.binaryType = 'arraybuffer';
    socket.addEventListener('message', (event) => {
      const message = decodeMessage(event.data);
      switch (message.type) {
        case MessageType.NumNames:
          state = {
            gameState: GameState.Submitting,
            numNames: message.content,
          };
          break;
        case MessageType.Names:
          state = {
            gameState: GameState.Playing,
            names: message.content[0],
            guesses: message.content[1],
          };
          break;
        case MessageType.NameGuessed:
          if (state.gameState === GameState.Playing) {
            state.guesses[message.content] = true;
          }
          break;
      }
    });
    socket.addEventListener(
      'close',
      () => (state = { gameState: GameState.Disconnected }),
    );
  });

  onDestroy(() => {
    if (socket.readyState === WebSocket.OPEN) {
      socket.close();
    }
  });

  function buttonClicked() {
    if (state.gameState === GameState.Playing) {
      socket.send(
        encodeMessage({ type: MessageType.StateSubmitting, content: null }),
      );
    } else if (state.gameState === GameState.Submitting) {
      socket.send(
        encodeMessage({ type: MessageType.StatePlaying, content: null }),
      );
    }
  }

  function nameClicked(index: number): () => void {
    return () => {
      socket.send(
        encodeMessage({ type: MessageType.MakeGuess, content: index }),
      );
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
        disabled={state.gameState === GameState.Disconnected ||
          (state.gameState === GameState.Submitting && state.numNames === 0)}
        onclick={buttonClicked}
      >
        {#if state.gameState === GameState.Disconnected}
          <Spinner width="1em" height="1em" />
          Loading
        {:else if state.gameState === GameState.Submitting}
          Show names
        {:else}
          Next round
        {/if}
      </button>
    </div>
  </header>
  <main class="flex min-h-0 grow flex-col overflow-y-auto text-center">
    <div class="flex grow flex-col justify-center p-4">
      {#if state.gameState === GameState.Disconnected}
        <Spinner width="150" height="150" />
      {:else if state.gameState === GameState.Submitting}
        <p class="text-3xl" in:scale>
          <span class="font-chewy text-6xl">{state.numNames}</span><br />
          names submitted
        </p>
      {:else}
        <ul class="flex flex-col gap-2 overflow-y-scroll text-3xl">
          {#each state.names as name, index (index)}
            <li in:fly|global={{ x: -200, opacity: 0, delay: index * 25 }}>
              <button
                class={[
                  'transition-colors-100 duration-50 hover:line-through',
                  state.guesses[index] && 'guessed',
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

<style>
  .guessed {
    text-decoration-line: line-through;
    color: var(--color-gray-500);
  }
</style>
