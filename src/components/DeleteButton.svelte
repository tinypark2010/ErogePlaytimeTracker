<script lang="ts">
  export let title: string;
  export let message: string;
  export let onconfirm: () => void | Promise<void>;
  export let label = '削除';
  export let ariaLabel: string | undefined = undefined;
  export let className = '';
  export let disabled = false;

  let confirmationOpen = false;
  let deleting = false;

  async function confirmDelete() {
    if (deleting) return;
    deleting = true;
    try {
      await onconfirm();
      confirmationOpen = false;
    } finally {
      deleting = false;
    }
  }
</script>

<button
  type="button"
  class={`danger ${className}`.trim()}
  {disabled}
  aria-label={ariaLabel ?? label}
  onclick={() => (confirmationOpen = true)}>{label}</button
>

{#if confirmationOpen}<div class="modal confirm-modal">
    <div
      class="panel confirm-box"
      role="alertdialog"
      aria-modal="true"
      aria-labelledby="delete-confirm-title"
      aria-describedby="delete-confirm-message"
    >
      <div class="confirm-icon">!</div>
      <h2 id="delete-confirm-title">{title}</h2>
      <p id="delete-confirm-message">{message}</p>
      <div class="confirm-actions">
        <button type="button" disabled={deleting} onclick={() => (confirmationOpen = false)}
          >キャンセル</button
        ><button class="danger" type="button" disabled={deleting} onclick={confirmDelete}
          >{deleting ? '削除中…' : '削除する'}</button
        >
      </div>
    </div>
  </div>{/if}
