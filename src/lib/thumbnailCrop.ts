export interface CropRect {
  x: number;
  y: number;
  width: number;
  height: number;
}

export type CropHandle = 'move' | 'nw' | 'n' | 'ne' | 'e' | 'se' | 's' | 'sw' | 'w';

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

export function fullCropRect(imageWidth: number, imageHeight: number): CropRect {
  return {
    x: 0,
    y: 0,
    width: Math.max(1, Math.round(imageWidth)),
    height: Math.max(1, Math.round(imageHeight)),
  };
}

export function fitCropPreview(
  imageWidth: number,
  imageHeight: number,
  availableWidth: number,
  availableHeight: number,
) {
  const width = Math.max(1, Math.round(imageWidth));
  const height = Math.max(1, Math.round(imageHeight));
  const scale = Math.min(
    1,
    Math.max(1, availableWidth) / width,
    Math.max(1, availableHeight) / height,
  );
  return {
    width: Math.max(1, Math.round(width * scale)),
    height: Math.max(1, Math.round(height * scale)),
  };
}

export function cropRectFromPoints(
  startX: number,
  startY: number,
  endX: number,
  endY: number,
  imageWidth: number,
  imageHeight: number,
): CropRect {
  const maximumWidth = Math.max(1, Math.round(imageWidth));
  const maximumHeight = Math.max(1, Math.round(imageHeight));
  const firstX = clamp(Math.round(startX), 0, maximumWidth);
  const firstY = clamp(Math.round(startY), 0, maximumHeight);
  const secondX = clamp(Math.round(endX), 0, maximumWidth);
  const secondY = clamp(Math.round(endY), 0, maximumHeight);
  let x = Math.min(firstX, secondX);
  let y = Math.min(firstY, secondY);
  let right = Math.max(firstX, secondX);
  let bottom = Math.max(firstY, secondY);
  if (right === x) {
    if (right < maximumWidth) right += 1;
    else x = Math.max(0, x - 1);
  }
  if (bottom === y) {
    if (bottom < maximumHeight) bottom += 1;
    else y = Math.max(0, y - 1);
  }
  return {
    x,
    y,
    width: right - x,
    height: bottom - y,
  };
}

export function composeCropRect(parent: CropRect, selection: CropRect): CropRect {
  return {
    x: parent.x + selection.x,
    y: parent.y + selection.y,
    width: selection.width,
    height: selection.height,
  };
}

export function transformCropRect(
  start: CropRect,
  handle: CropHandle,
  deltaX: number,
  deltaY: number,
  imageWidth: number,
  imageHeight: number,
): CropRect {
  const maximumWidth = Math.max(1, Math.round(imageWidth));
  const maximumHeight = Math.max(1, Math.round(imageHeight));
  const dx = Math.round(deltaX);
  const dy = Math.round(deltaY);
  if (handle === 'move') {
    return {
      ...start,
      x: clamp(start.x + dx, 0, maximumWidth - start.width),
      y: clamp(start.y + dy, 0, maximumHeight - start.height),
    };
  }

  const startRight = start.x + start.width;
  const startBottom = start.y + start.height;
  const left = handle.includes('w') ? clamp(start.x + dx, 0, startRight - 1) : start.x;
  const right = handle.includes('e')
    ? clamp(startRight + dx, start.x + 1, maximumWidth)
    : startRight;
  const top = handle.includes('n') ? clamp(start.y + dy, 0, startBottom - 1) : start.y;
  const bottom = handle.includes('s')
    ? clamp(startBottom + dy, start.y + 1, maximumHeight)
    : startBottom;
  return {
    x: left,
    y: top,
    width: right - left,
    height: bottom - top,
  };
}
