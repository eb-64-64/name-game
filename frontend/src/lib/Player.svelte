<script lang="ts">
  import { onMount } from 'svelte';
  import { encodeMessage, MessageType, parseMessage } from './messages';

  let socket: WebSocket;
  onMount(() => {
    socket = new WebSocket('/ws/player');
    socket.binaryType = 'arraybuffer';
    socket.addEventListener('open', (event) => {
      socket.send(
        encodeMessage({ type: MessageType.SubmissionTime, content: null }),
      );
      socket.send(
        encodeMessage({ type: MessageType.Submission, content: 'Emma' }),
      );
      socket.send(encodeMessage({ type: MessageType.PlayTime, content: null }));
      socket.send(
        encodeMessage({
          type: MessageType.SubmissionList,
          content: ['Emma', 'Esther'],
        }),
      );
    });
    socket.addEventListener('message', (event) => {
      const message = parseMessage(event.data);
      console.log(message);
    });
  });
</script>

<div class="flex h-screen flex-col justify-center">
  <div class="bg-surface-950 mx-auto w-2/3 min-w-xs space-y-16 rounded-xl p-16">
    <form class="flex flex-col gap-8 text-center">
      <h1 class="font-modak text-4xl">Submit a name!</h1>
      <input
        type="text"
        placeholder="Name"
        class="input w-full p-2 text-center"
      />
      <input type="submit" class="btn preset-filled-primary-500 w-full p-2" />
    </form>
  </div>
</div>
