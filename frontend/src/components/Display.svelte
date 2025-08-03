<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { MessageType } from '../lib/messages';
  import { scale } from 'svelte/transition';
  import { GameState } from '../lib/state';
  import { ReconnectingSocket } from '../lib/reconnecting-socket';
  import DisconnectionToast from './DisconnectionToast.svelte';
  import NameList from './NameList.svelte';

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
        case MessageType.NameUnguessed:
          if (gameState.state === GameState.Playing) {
            gameState.guesses[message.content] = false;
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
      socket.send({ type: MessageType.RequestSubmittingState, content: null });
    } else if (gameState.state === GameState.Submitting) {
      socket.send({ type: MessageType.RequestPlayingState, content: null });
    }
  }

  function nameClicked(index: number) {
    if (gameState.state === GameState.Playing) {
      if (gameState.guesses[index]) {
        socket.send({ type: MessageType.UnguessName, content: index });
      } else {
        socket.send({ type: MessageType.GuessName, content: index });
      }
    }
  }
</script>

<div class="flex h-full flex-col">
  <header
    class="border-surface-500 bg-surface-50-950 sticky top-0 border-b-[0.25px] p-8 text-center"
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
  <main class="flex grow flex-col text-center">
    <div class="flex grow flex-col justify-center p-4">
      {#if gameState.state === GameState.Submitting}
        <p class="text-3xl" in:scale>
          <span class="font-chewy text-6xl">{gameState.numNames}</span><br />
          names submitted
        </p>
      {:else}
        <NameList
          names={gameState.names}
          guesses={gameState.guesses}
          clickability={{ clickable: true, onClick: nameClicked }}
        />
      {/if}
    </div>
  </main>
</div>

<DisconnectionToast {connected} />
