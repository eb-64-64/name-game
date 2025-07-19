<script lang="ts">
  import { onMount } from 'svelte';
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
  <div class="bg-surface-950 mx-auto w-2/3 min-w-xs space-y-16 rounded-xl p-16">
    <form class="flex flex-col gap-8 text-center" onsubmit={onSubmit}>
      <h1 class="font-modak text-4xl">Submit a name!</h1>
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
