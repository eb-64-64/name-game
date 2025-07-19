<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import { encodeMessage, MessageType, parseMessage } from './messages';

  let enabled = $state(false);

  let name = $state('');

  let socket: WebSocket;
  onMount(() => {
    socket = new WebSocket('/ws/player');
    socket.binaryType = 'arraybuffer';
    socket.addEventListener('message', (event) => {
      const message = parseMessage(event.data);
      switch (message.type) {
        case MessageType.SubmissionTime:
          enabled = true;
          break;
        case MessageType.PlayTime:
          enabled = false;
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

  function onSubmit(event: SubmitEvent) {
    event.preventDefault();
    if (name && enabled) {
      socket.send(
        encodeMessage({
          type: MessageType.Submission,
          content: name,
        }),
      );
      name = '';
    }
  }
</script>

<div class="flex h-screen flex-col justify-center">
  <div class="bg-surface-950 mx-auto w-2/3 min-w-xs rounded-xl p-16">
    <form class="flex flex-col gap-8 text-center" onsubmit={onSubmit}>
      <h1 class="font-chewy text-4xl">The Name Game!</h1>
      <input
        bind:value={name}
        class="input w-full p-2 text-center"
        id="name"
        placeholder="Name"
        type="text"
      />
      <input
        class="btn preset-filled-primary-500 w-full p-2"
        disabled={!enabled}
        type="submit"
      />
    </form>
  </div>
</div>
