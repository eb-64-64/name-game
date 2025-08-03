<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { MessageType } from '../lib/messages';
  import { GameState } from '../lib/state';
  import DisconnectionToast from './DisconnectionToast.svelte';
  import { ReconnectingSocket } from '../lib/reconnecting-socket';

  let connected = $state(true);
  let gameState = $state(GameState.Submitting);

  let name = $state('');
  let names = $state([
    'Mom',
    'Dad',
    'Emma',
    'Esther',
    'Lydia',
    'Hannah',
    'Eve',
    'Mom',
    'Dad',
    'Emma',
    'Esther',
    'Lydia',
    'Hannah',
    'Eve',
  ]);

  let socket: ReconnectingSocket;
  onMount(() => {
    socket = new ReconnectingSocket('/ws/player');
    socket.onOpen = () => {
      connected = true;
    };
    socket.onMessage = (message) => {
      switch (message.type) {
        case MessageType.StateSubmitting:
          gameState = GameState.Submitting;
          break;
        case MessageType.StatePlaying:
          gameState = GameState.Playing;
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

  function onSubmit(event: SubmitEvent) {
    event.preventDefault();
    if (name && gameState === GameState.Submitting) {
      socket.send({
        type: MessageType.SubmitName,
        content: name,
      });
      name = '';
    }
  }
</script>

<header
  class="border-surface-500 bg-surface-50-950 sticky top-0 border-b-[0.25px] p-8 text-center"
>
  <h1 class="font-chewy text-4xl">The Name Game!</h1>
</header>

<main class="mx-auto max-w-3xl divide-y-[0.25px] text-center">
  <div
    class="border-surface-500 bg-(--body-background-color) p-8 dark:bg-(--body-background-color-dark)"
  >
    <form class="mx-auto max-w-3xl" onsubmit={onSubmit}>
      <div class="input-group grid-cols-[1fr_auto]">
        <input
          autocomplete="off"
          bind:value={name}
          class="ig-input p-2 text-center"
          id="name"
          name="name"
          placeholder="Name"
          type="text"
        />
        <input
          class="ig-btn preset-filled-primary-500 transition-colors-100 p-2 px-6"
          disabled={!connected || gameState !== GameState.Submitting}
          type="submit"
        />
      </div>
    </form>
  </div>
  <ul class="items-middle flex flex-col gap-6 p-8 text-lg">
    {#each names as name}
      <li class="bg-primary-500 mx-auto flex w-fit gap-4 rounded-lg px-4 py-1">
        <span>{name}</span>
        <button class="font-extrabold">&times;</button>
      </li>
    {/each}
  </ul>
</main>

<DisconnectionToast {connected} />
