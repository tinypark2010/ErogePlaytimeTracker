export function screenshotNavigation<T extends { id: number }>(
  screenshots: readonly T[],
  selectedId: number | null,
) {
  const index = selectedId === null ? -1 : screenshots.findIndex((shot) => shot.id === selectedId);
  return {
    index,
    previous: index > 0 ? screenshots[index - 1] : null,
    next: index >= 0 ? (screenshots[index + 1] ?? null) : null,
  };
}
