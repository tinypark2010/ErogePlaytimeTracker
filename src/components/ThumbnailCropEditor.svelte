<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { api } from '../lib/api';
  import { userErrorMessage } from '../lib/errors';
  import { imageSrc } from '../lib/time';
  import {
    composeCropRect,
    cropRectFromPoints,
    fitCropPreview,
    fullCropRect,
    transformCropRect,
    type CropHandle,
    type CropRect,
  } from '../lib/thumbnailCrop';

  export let imagePath: string;
  export let ondone: (path: string) => void | Promise<void>;
  export let onremove: (() => void) | undefined = undefined;
  export let onselect: (() => void) | undefined = undefined;
  export let onbusy: (busy: boolean) => void = () => {};
  export let selecting = false;
  export let saveDisabled = false;

  const cropHandles: CropHandle[] = ['nw', 'n', 'ne', 'e', 'se', 's', 'sw', 'w'];
  let cropLoading = true,
    cropSaving = false,
    cropCanvas: HTMLCanvasElement | null = null,
    cropWorkspace: HTMLDivElement | null = null,
    cropImage: HTMLImageElement | null = null,
    cropPreviewWidth = 1,
    cropPreviewHeight = 1,
    cropRect: CropRect = { x: 0, y: 0, width: 1, height: 1 },
    cropDraftRect: CropRect = { x: 0, y: 0, width: 1, height: 1 },
    cropChanged = false,
    cropError = '';
  let cropDraftChanged: boolean;
  let cropWorkspaceObserver: ResizeObserver | null = null;
  let cropTransformCleanup: (() => void) | null = null;

  $: cropDraftChanged =
    cropDraftRect.x !== 0 ||
    cropDraftRect.y !== 0 ||
    cropDraftRect.width !== cropRect.width ||
    cropDraftRect.height !== cropRect.height;

  onMount(() => {
    void initialize();
    return stopCropInteractions;
  });

  function loadCropImage(path: string) {
    return new Promise<HTMLImageElement>((resolve, reject) => {
      const image = new Image();
      image.crossOrigin = 'anonymous';
      image.onload = () => resolve(image);
      image.onerror = () => reject(new Error('サムネイル画像を読み込めませんでした'));
      image.src = imageSrc(path);
    });
  }
  async function initialize() {
    await tick();
    stopCropWorkspaceObserver();
    if (cropWorkspace) {
      cropWorkspaceObserver = new ResizeObserver(() => {
        if (cropImage) void renderCropPreview();
      });
      cropWorkspaceObserver.observe(cropWorkspace);
    }
    try {
      cropImage = await loadCropImage(imagePath);
      await resetCrop();
    } catch (e) {
      cropError = userErrorMessage(e, 'サムネイル画像を読み込めませんでした。');
    } finally {
      cropLoading = false;
    }
  }
  function drawCropPreview() {
    if (!cropCanvas || !cropImage) return;
    const context = cropCanvas.getContext('2d');
    if (!context) return;
    context.clearRect(0, 0, cropPreviewWidth, cropPreviewHeight);
    context.drawImage(
      cropImage,
      cropRect.x,
      cropRect.y,
      cropRect.width,
      cropRect.height,
      0,
      0,
      cropPreviewWidth,
      cropPreviewHeight,
    );
  }
  async function renderCropPreview() {
    if (!cropImage || !cropWorkspace) return;
    const workspaceStyle = getComputedStyle(cropWorkspace);
    const availableWidth =
      cropWorkspace.clientWidth -
      Number.parseFloat(workspaceStyle.paddingLeft) -
      Number.parseFloat(workspaceStyle.paddingRight);
    const availableHeight =
      cropWorkspace.clientHeight -
      Number.parseFloat(workspaceStyle.paddingTop) -
      Number.parseFloat(workspaceStyle.paddingBottom);
    const preview = fitCropPreview(
      cropRect.width,
      cropRect.height,
      Math.min(640, availableWidth),
      Math.min(520, availableHeight),
    );
    cropPreviewWidth = preview.width;
    cropPreviewHeight = preview.height;
    await tick();
    drawCropPreview();
  }
  async function resetCrop() {
    if (!cropImage) return;
    cropRect = fullCropRect(cropImage.naturalWidth, cropImage.naturalHeight);
    cropDraftRect = fullCropRect(cropRect.width, cropRect.height);
    cropChanged = false;
    await renderCropPreview();
  }
  function stopCropWorkspaceObserver() {
    cropWorkspaceObserver?.disconnect();
    cropWorkspaceObserver = null;
  }
  function stopCropInteractions() {
    cropTransformCleanup?.();
    stopCropWorkspaceObserver();
  }
  function beginCropTransform(event: PointerEvent, handle: CropHandle) {
    if (!cropCanvas || !cropImage) return;
    event.preventDefault();
    event.stopPropagation();
    const stage = (event.currentTarget as HTMLElement).closest(
      '.thumbnail-crop-stage',
    ) as HTMLElement | null;
    if (!stage) return;
    cropTransformCleanup?.();
    const imageWidth = cropRect.width;
    const imageHeight = cropRect.height;
    const startX = event.clientX;
    const startY = event.clientY;
    const startRect = { ...cropDraftRect };
    const creatingNewSelection =
      handle === 'move' &&
      startRect.x === 0 &&
      startRect.y === 0 &&
      startRect.width === imageWidth &&
      startRect.height === imageHeight;
    const bounds = stage.getBoundingClientRect();
    const anchorX = ((event.clientX - bounds.left) / bounds.width) * imageWidth;
    const anchorY = ((event.clientY - bounds.top) / bounds.height) * imageHeight;

    const move = (moveEvent: PointerEvent) => {
      if (creatingNewSelection) {
        const currentBounds = stage.getBoundingClientRect();
        const currentX =
          ((moveEvent.clientX - currentBounds.left) / currentBounds.width) * imageWidth;
        const currentY =
          ((moveEvent.clientY - currentBounds.top) / currentBounds.height) * imageHeight;
        cropDraftRect = cropRectFromPoints(
          anchorX,
          anchorY,
          currentX,
          currentY,
          imageWidth,
          imageHeight,
        );
        return;
      }
      cropDraftRect = transformCropRect(
        startRect,
        handle,
        ((moveEvent.clientX - startX) / stage.clientWidth) * imageWidth,
        ((moveEvent.clientY - startY) / stage.clientHeight) * imageHeight,
        imageWidth,
        imageHeight,
      );
    };
    const cleanup = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', end);
      window.removeEventListener('pointercancel', cancel);
      cropTransformCleanup = null;
    };
    const end = () => cleanup();
    const cancel = () => {
      cleanup();
      cropDraftRect = fullCropRect(cropRect.width, cropRect.height);
    };
    cropTransformCleanup = cleanup;
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', end, { once: true });
    window.addEventListener('pointercancel', cancel, { once: true });
  }
  async function commitCropDraft() {
    if (!cropImage || !cropDraftChanged) return;
    cropRect = composeCropRect(cropRect, cropDraftRect);
    cropDraftRect = fullCropRect(cropRect.width, cropRect.height);
    cropChanged = true;
    await renderCropPreview();
  }
  async function saveCrop() {
    if (!cropImage || cropSaving) return;
    cropSaving = true;
    onbusy(true);
    cropError = '';
    try {
      let savedPath = imagePath;
      if (cropChanged) {
        const output = document.createElement('canvas');
        output.width = cropRect.width;
        output.height = cropRect.height;
        const context = output.getContext('2d');
        if (!context) throw new Error('トリミング画像を生成できませんでした');
        context.drawImage(
          cropImage,
          cropRect.x,
          cropRect.y,
          cropRect.width,
          cropRect.height,
          0,
          0,
          cropRect.width,
          cropRect.height,
        );
        const encoded = output.toDataURL('image/png').split(',', 2)[1];
        savedPath = await api.saveCroppedThumbnail(encoded);
      }
      await ondone(savedPath);
    } catch (e) {
      cropError = userErrorMessage(e, 'サムネイル画像を保存できませんでした。');
    } finally {
      cropSaving = false;
      onbusy(false);
    }
  }
</script>

<div class="thumbnail-crop-editor">
  <div class="thumbnail-crop-workspace" bind:this={cropWorkspace}>
    {#if onremove}<div class="thumbnail-crop-remove-control">
        <button
          type="button"
          aria-label="サムネイルの選択を解除"
          disabled={cropLoading || cropSaving || selecting}
          onclick={onremove}
        >
          <svg viewBox="0 0 24 24" aria-hidden="true">
            <path d="m7 7 10 10M17 7 7 17" />
          </svg>
        </button>
      </div>{/if}
    {#if cropImage}<div
        class="thumbnail-crop-stage"
        style={`width:${cropPreviewWidth}px;aspect-ratio:${cropPreviewWidth}/${cropPreviewHeight}`}
      >
        <canvas
          bind:this={cropCanvas}
          width={cropPreviewWidth}
          height={cropPreviewHeight}
          aria-hidden="true"
        ></canvas>
        <div
          class="thumbnail-crop-selection"
          style={`left:${(cropDraftRect.x / cropRect.width) * 100}%;top:${(cropDraftRect.y / cropRect.height) * 100}%;width:${(cropDraftRect.width / cropRect.width) * 100}%;height:${(cropDraftRect.height / cropRect.height) * 100}%`}
        >
          <button
            type="button"
            class="thumbnail-crop-move"
            aria-label="トリミング範囲を移動"
            onpointerdown={(event) => beginCropTransform(event, 'move')}
          ></button>
          {#each cropHandles as handle}<button
              type="button"
              class="thumbnail-crop-handle {handle}"
              aria-label="トリミング範囲のサイズを変更"
              onpointerdown={(event) => beginCropTransform(event, handle)}
            ></button>{/each}
        </div>
      </div>{:else if cropLoading}<span>画像を読み込み中…</span>{/if}
  </div>
  {#if cropImage}<p class="thumbnail-crop-size">
      選択範囲: {cropDraftRect.width} × {cropDraftRect.height} px
    </p>{/if}
  {#if cropError}<p class="error">{cropError}</p>{/if}
  <div class="actions thumbnail-crop-actions">
    {#if onselect}<button
        disabled={cropLoading || cropSaving || selecting || !cropImage}
        onclick={resetCrop}>リセット</button
      >
      <button
        disabled={cropLoading || cropSaving || selecting || !cropImage || !cropDraftChanged}
        onclick={commitCropDraft}>トリミング</button
      >
      <button type="button" disabled={cropLoading || cropSaving || selecting} onclick={onselect}
        >{selecting ? '取込中…' : '画像を選択…'}</button
      >{:else}<button disabled={cropLoading || cropSaving || !cropImage} onclick={resetCrop}
        >リセット</button
      >
      <button
        disabled={cropLoading || cropSaving || !cropImage || !cropDraftChanged}
        onclick={commitCropDraft}>トリミング</button
      >{/if}
    <button
      class="primary"
      disabled={cropLoading ||
        cropSaving ||
        selecting ||
        !cropImage ||
        cropDraftChanged ||
        (saveDisabled && !cropChanged)}
      onclick={saveCrop}>{cropSaving ? '保存中…' : '保存'}</button
    >
  </div>
</div>
