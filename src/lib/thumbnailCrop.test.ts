import { describe, expect, it } from 'vitest';
import {
  composeCropRect,
  cropRectFromPoints,
  fitCropPreview,
  fullCropRect,
  transformCropRect,
} from './thumbnailCrop';

describe('thumbnail crop rectangle', () => {
  it('defaults to the original image dimensions', () => {
    expect(fullCropRect(1600, 900)).toEqual({ x: 0, y: 0, width: 1600, height: 900 });
  });

  it('fits large images within the available modal area without changing their ratio', () => {
    expect(fitCropPreview(2000, 3000, 640, 400)).toEqual({ width: 267, height: 400 });
    expect(fitCropPreview(3000, 2000, 500, 520)).toEqual({ width: 500, height: 333 });
    expect(fitCropPreview(320, 240, 640, 520)).toEqual({ width: 320, height: 240 });
  });

  it('allows a freeform crop from any edge', () => {
    const start = fullCropRect(1600, 900);

    expect(transformCropRect(start, 'nw', 125, 80, 1600, 900)).toEqual({
      x: 125,
      y: 80,
      width: 1475,
      height: 820,
    });
  });

  it('creates a crop by dragging in either direction', () => {
    expect(cropRectFromPoints(100, 80, 500, 400, 1600, 900)).toEqual({
      x: 100,
      y: 80,
      width: 400,
      height: 320,
    });
    expect(cropRectFromPoints(500, 400, 100, 80, 1600, 900)).toEqual({
      x: 100,
      y: 80,
      width: 400,
      height: 320,
    });
  });

  it('applies another crop within the current crop', () => {
    expect(
      composeCropRect(
        { x: 100, y: 80, width: 800, height: 600 },
        { x: 50, y: 25, width: 400, height: 300 },
      ),
    ).toEqual({ x: 150, y: 105, width: 400, height: 300 });
  });

  it('keeps resizing and movement inside the source image', () => {
    const start = { x: 100, y: 100, width: 400, height: 300 };

    expect(transformCropRect(start, 'move', 1000, -1000, 800, 600)).toEqual({
      x: 400,
      y: 0,
      width: 400,
      height: 300,
    });
    expect(transformCropRect(start, 'se', 1000, 1000, 800, 600)).toEqual({
      x: 100,
      y: 100,
      width: 700,
      height: 500,
    });
  });

  it('never collapses the crop below one pixel', () => {
    const start = { x: 10, y: 20, width: 100, height: 80 };

    expect(transformCropRect(start, 'nw', 1000, 1000, 500, 500)).toEqual({
      x: 109,
      y: 99,
      width: 1,
      height: 1,
    });
  });
});
