<script lang="ts">
  let { width = undefined, height = undefined } = $props();
</script>

<svg
  xmlns="http://www.w3.org/2000/svg"
  viewBox="0 0 100 100"
  {width}
  {height}
  class="mx-auto animate-spin"
>
  <circle
    stroke-linecap="round"
    fill="none"
    stroke-dasharray="62.831853"
    class="stroke-surface-900-100"
    stroke-width="10"
    r="40"
    cy="50"
    cx="50"
  >
  </circle>
</svg>
