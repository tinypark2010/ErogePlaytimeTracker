<script lang="ts">
  import { duration } from '../lib/time';

  export let label: string;
  export let seconds: number;
  export let selected = false;
  export let recording = false;
  export let disabled = false;
  export let onselect: () => void;
</script>

<button
  type="button"
  class="history-data-row"
  class:selected
  class:recording
  {disabled}
  onclick={onselect}
>
  <span>{label}</span>
  <strong>{duration(seconds)}</strong>
</button>
