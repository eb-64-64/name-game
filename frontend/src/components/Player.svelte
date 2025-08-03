<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { MessageType } from '../lib/messages';
  import { GameState } from '../lib/state';
  import DisconnectionToast from './DisconnectionToast.svelte';
  import { ReconnectingSocket } from '../lib/reconnecting-socket';

  let connected = $state(true);
  let gameState = $state(GameState.Submitting);

  let name = $state('');

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

<div class="flex h-screen flex-col justify-center">
  <main
    class="bg-surface-50-950/75 border-surface-100-900/30 mx-auto w-2/3 min-w-xs rounded-xl border p-16"
  >
    <form class="flex flex-col gap-8 text-center" onsubmit={onSubmit}>
      <h1 class="font-chewy text-4xl">The Name Game!</h1>
      <input
        autocomplete="off"
        bind:value={name}
        class="input w-full p-2 text-center"
        id="name"
        name="name"
        placeholder="Name"
        type="text"
      />
      <input
        class="btn preset-filled-primary-500 transition-colors-100 w-full p-2"
        disabled={!connected || gameState !== GameState.Submitting}
        type="submit"
      />
    </form>
  </main>
</div>

<DisconnectionToast {connected} />
