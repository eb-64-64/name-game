<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { MessageType, type Uuid } from '../lib/messages';
  import { GameState } from '../lib/state';
  import DisconnectionToast from './DisconnectionToast.svelte';
  import { ReconnectingSocket } from '../lib/reconnecting-socket';
  import NameList from './NameList.svelte';
  import { X } from '@lucide/svelte';
  import { clearNames, getNames, setNames } from '../lib/storage';
  import { scale } from 'svelte/transition';

  let connected = $state(true);
  let gameState:
    | {
        state: GameState.Submitting;
        epoch: number;
        names: [string, Uuid][];
      }
    | { state: GameState.Playing; names: string[]; guesses: boolean[] } =
    $state({
      state: GameState.Submitting,
      epoch: -1,
      names: [],
    });

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
          gameState = {
            state: GameState.Submitting,
            epoch: message.content,
            names: getNames(message.content),
          };
          break;
        case MessageType.NameSubmitted:
          if (gameState.state === GameState.Submitting) {
            gameState.names.push(message.content);
            setNames(gameState.epoch, gameState.names);
          }
          break;
        case MessageType.NameUnsubmitted:
          if (gameState.state === GameState.Submitting) {
            const index = gameState.names.findIndex(
              ([, id]) => id === message.content,
            );
            gameState.names.splice(index, 1);
            setNames(gameState.epoch, gameState.names);
          }
          break;
        case MessageType.Names:
          gameState = {
            state: GameState.Playing,
            names: message.content[0],
            guesses: message.content[1],
          };
          clearNames();
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

  function onSubmit(event: SubmitEvent) {
    event.preventDefault();
    if (name && gameState.state === GameState.Submitting) {
      socket.send({
        type: MessageType.SubmitName,
        content: name,
      });
      name = '';
    }
  }

  function unsubmitName(id: Uuid) {
    socket.send({ type: MessageType.UnsubmitName, content: id });
  }
</script>

<div class="flex h-full flex-col">
  <header
    class="border-surface-500 bg-surface-50-950 sticky top-0 border-b-[0.25px] p-8 text-center"
  >
    <h1 class="font-chewy text-4xl">The Name Game!</h1>
  </header>

  <main class="mx-auto w-full divide-y-[0.25px] text-center">
    {#if gameState.state === GameState.Submitting}
      <div
        class="border-surface-500 mx-auto max-w-3xl bg-(--body-background-color) p-8 dark:bg-(--body-background-color-dark)"
      >
        <form onsubmit={onSubmit}>
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
              disabled={!connected || gameState.state !== GameState.Submitting}
              type="submit"
            />
          </div>
        </form>
      </div>
      <ul class="items-middle flex flex-col gap-6 p-8 text-lg">
        {#each gameState.names as [name, id] (id)}
          <li
            class="bg-primary-500 mx-auto flex w-fit gap-3 rounded-lg px-4 py-1"
            transition:scale
          >
            <span>{name}</span>
            <button onclick={() => unsubmitName(id)}><X /></button>
          </li>
        {/each}
      </ul>
    {:else}
      <div class="m-6 flex flex-col">
        <NameList
          names={gameState.names}
          guesses={gameState.guesses}
          clickability={{ clickable: false }}
        />
      </div>
    {/if}
  </main>
</div>

<DisconnectionToast {connected} />
