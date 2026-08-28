<script lang="ts">
  import ThumbnailCropEditor from './ThumbnailCropEditor.svelte';

  export let imagePath: string;
  export let ondone: (path: string) => void | Promise<void>;
  export let onclose: () => void;

  let saving = false;
</script>

<div class="modal thumbnail-crop-modal">
  <div
    class="panel thumbnail-cropper"
    role="dialog"
    tabindex="-1"
    aria-modal="true"
    aria-labelledby="thumbnail-crop-title"
  >
    <button class="close" aria-label="閉じる" disabled={saving} onclick={onclose}>×</button>
    <h2 id="thumbnail-crop-title">サムネイルをトリミング</h2>
    <ThumbnailCropEditor {imagePath} {ondone} onbusy={(busy) => (saving = busy)} />
  </div>
</div>
