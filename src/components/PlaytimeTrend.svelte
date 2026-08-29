<script lang="ts">
  import { onMount } from 'svelte';
  import {
    compactDuration,
    dailyTrend,
    formatDateKey,
    nextChartZoom,
    type DailyTrendPoint,
  } from '../lib/statistics';
  import { imageSrc } from '../lib/time';
  import type { StatisticsDay, StatisticsPeriodKind } from '../lib/types';

  export let days: StatisticsDay[] = [];
  export let kind: StatisticsPeriodKind;

  const width = 900;
  const plotTop = 14;
  const plotBottom = 140;
  const plotHeight = plotBottom - plotTop;
  const gameColors = [
    '#9b7be8',
    '#d06f8d',
    '#df9248',
    '#4da3cf',
    '#7cab55',
    '#b96bc2',
    '#d0ad3d',
    '#607fd0',
    '#cb685e',
    '#4a9b83',
  ];
  let chartElement: HTMLDivElement;
  let viewportElement: HTMLDivElement;
  let tooltipElement: HTMLDivElement;
  let zoom = 1;
  let hoverIndex: number | null = null;
  let pinnedIndex: number | null = null;
  let activeIndex: number | null = null;
  let currentDays = days;

  $: points = dailyTrend(days);
  $: if (days !== currentDays) {
    currentDays = days;
    hoverIndex = null;
    pinnedIndex = null;
    zoom = 1;
    if (viewportElement) viewportElement.scrollLeft = 0;
  }
  $: activeIndex = pinnedIndex ?? hoverIndex;
  $: maximumZoom = Math.min(32, Math.max(4, points.length / 45));
  $: if (zoom > maximumZoom) zoom = maximumZoom;
  $: chartWidth = width * zoom;
  $: maximumDaily = Math.max(1, ...points.map((point) => point.playtime_seconds));
  $: maximumCumulative = Math.max(1, ...points.map((point) => point.cumulative_seconds));
  $: labelStep = Math.max(1, Math.ceil(points.length / (8 * zoom)));
  $: barStrokeWidth = Math.max(
    0.45,
    Math.min(18, (chartWidth / Math.max(1, points.length)) * 0.68),
  );
  $: stackedBars = buildStackedBars(points, maximumDaily, chartWidth);
  $: cumulativePoints = buildCumulativePoints(points, maximumCumulative, chartWidth);
  $: activePoint = activeIndex === null ? null : (points[activeIndex] ?? null);
  $: activeGames = activePoint?.games ?? [];
  $: accessibleIndex = activeIndex ?? Math.max(0, points.length - 1);
  $: accessiblePoint = points[accessibleIndex];
  $: accessibleValue = accessiblePoint
    ? `${formatDateKey(accessiblePoint.date)}、合計${compactDuration(accessiblePoint.playtime_seconds)}、累計${compactDuration(accessiblePoint.cumulative_seconds)}`
    : 'データなし';
  $: tooltipPosition =
    activeIndex === null ? 50 : ((activeIndex + 0.5) / Math.max(1, points.length)) * 100;

  onMount(() => {
    function handleDocumentClick(event: MouseEvent) {
      if (pinnedIndex === null || !(event.target instanceof Node)) return;
      if (tooltipElement?.contains(event.target) || chartElement?.contains(event.target)) return;
      pinnedIndex = null;
      hoverIndex = null;
    }

    document.addEventListener('click', handleDocumentClick);
    viewportElement.addEventListener('wheel', handleWheel, { passive: false });
    return () => {
      document.removeEventListener('click', handleDocumentClick);
      viewportElement.removeEventListener('wheel', handleWheel);
    };
  });

  function pointX(index: number, pointCount: number, graphWidth: number) {
    return ((index + 0.5) / Math.max(1, pointCount)) * graphWidth;
  }

  function gameColor(gameId: number) {
    return gameColors[Math.abs(Math.trunc(gameId)) % gameColors.length];
  }

  function buildStackedBars(points: DailyTrendPoint[], maximum: number, graphWidth: number) {
    const bars: Array<{ x: number; top: number; bottom: number; color: string }> = [];
    points.forEach((point, index) => {
      let stackedSeconds = 0;
      point.games.forEach((game) => {
        const bottom = plotBottom - (stackedSeconds / maximum) * plotHeight;
        stackedSeconds += game.playtime_seconds;
        bars.push({
          x: pointX(index, points.length, graphWidth),
          top: plotBottom - (stackedSeconds / maximum) * plotHeight,
          bottom,
          color: gameColor(game.game_id),
        });
      });
    });
    return bars;
  }

  function buildCumulativePoints(points: DailyTrendPoint[], maximum: number, graphWidth: number) {
    return points
      .map((point, index) => {
        const x = pointX(index, points.length, graphWidth);
        const y = plotBottom - (point.cumulative_seconds / maximum) * plotHeight;
        return `${x},${y}`;
      })
      .join(' ');
  }

  function axisLabel(date: string) {
    const year = Number(date.slice(0, 4));
    const month = Number(date.slice(5, 7));
    const day = Number(date.slice(8, 10));
    if (kind === 'month') return `${day}日`;
    if (kind === 'year') return `${month}/${day}`;
    return `${year}/${month}`;
  }

  function indexFromPointer(event: PointerEvent | MouseEvent) {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = Math.min(0.999999, Math.max(0, (event.clientX - bounds.left) / bounds.width));
    return Math.floor(ratio * points.length);
  }

  function selectFromPointer(event: PointerEvent) {
    if (!points.length || pinnedIndex !== null) return;
    hoverIndex = indexFromPointer(event);
  }

  function handleWheel(event: WheelEvent) {
    if (
      pinnedIndex !== null &&
      event.target instanceof Node &&
      tooltipElement?.contains(event.target)
    ) {
      if (event.target instanceof Element) {
        const list = event.target.closest('.statistics-chart-tooltip ul');
        if (list && list.scrollHeight > list.clientHeight) return;
      }
      event.preventDefault();
      return;
    }
    const nextZoom = nextChartZoom(zoom, maximumZoom, event.deltaY, event.deltaMode);
    event.preventDefault();
    if (Math.abs(nextZoom - zoom) < 0.001) return;

    const bounds = viewportElement.getBoundingClientRect();
    const cursorX = event.clientX - bounds.left;
    const anchor = (viewportElement.scrollLeft + cursorX) / viewportElement.scrollWidth;
    zoom = nextZoom;
    requestAnimationFrame(() => {
      viewportElement.scrollLeft = anchor * viewportElement.scrollWidth - cursorX;
    });
  }

  function handleChartClick(event: MouseEvent) {
    if (!points.length) return;
    const index = indexFromPointer(event);
    if (pinnedIndex !== null) {
      pinnedIndex = null;
      hoverIndex = index;
      return;
    }
    pinnedIndex = index;
    hoverIndex = null;
  }

  function handleKey(event: KeyboardEvent) {
    if (event.key === 'Escape' && pinnedIndex !== null) {
      event.preventDefault();
      hoverIndex = pinnedIndex;
      pinnedIndex = null;
      return;
    }
    if (!points.length) return;
    if (['Enter', ' '].includes(event.key)) {
      event.preventDefault();
      if (pinnedIndex === null) {
        pinnedIndex = activeIndex ?? points.length - 1;
        hoverIndex = null;
      }
      return;
    }
    if (pinnedIndex !== null || !['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key))
      return;
    event.preventDefault();
    if (event.key === 'Home') hoverIndex = 0;
    else if (event.key === 'End') hoverIndex = points.length - 1;
    else if (event.key === 'ArrowLeft')
      hoverIndex = Math.max(0, (activeIndex ?? points.length) - 1);
    else hoverIndex = Math.min(points.length - 1, (activeIndex ?? -1) + 1);
  }
</script>

<div class="statistics-combined-chart">
  <div bind:this={viewportElement} class:zoomed={zoom > 1.01} class="statistics-chart-viewport">
    <div class="statistics-chart-stage" style={`width: ${zoom * 100}%`}>
      <div
        bind:this={chartElement}
        class="statistics-chart-interaction"
        role="slider"
        tabindex="0"
        aria-label="グラフの日付"
        aria-valuemin="0"
        aria-valuemax={Math.max(0, points.length - 1)}
        aria-valuenow={accessibleIndex}
        aria-valuetext={accessibleValue}
        onpointermove={selectFromPointer}
        onpointerleave={() => {
          if (pinnedIndex === null) hoverIndex = null;
        }}
        onfocus={() => {
          if (activeIndex === null && points.length) hoverIndex = points.length - 1;
        }}
        onkeydown={handleKey}
        onclick={handleChartClick}
      >
        <svg
          class="statistics-chart"
          viewBox={`0 0 ${chartWidth} 180`}
          role="img"
          aria-label="日別プレイ時間と累計プレイ時間を重ねたグラフ"
        >
          <line
            class="statistics-chart-axis"
            x1="0"
            y1={plotBottom}
            x2={chartWidth}
            y2={plotBottom}
          />
          <line
            class="statistics-chart-gridline"
            x1="0"
            y1={plotTop + plotHeight / 2}
            x2={chartWidth}
            y2={plotTop + plotHeight / 2}
          />
          {#each stackedBars as bar}
            <line
              class="statistics-chart-bar-segment"
              x1={bar.x}
              y1={bar.bottom}
              x2={bar.x}
              y2={bar.top}
              style={`stroke: ${bar.color}; stroke-width: ${barStrokeWidth}`}
            />
          {/each}
          {#if points.length}
            <polyline class="statistics-chart-line" points={cumulativePoints} />
          {/if}
          {#each points as point, index}
            {#if index % labelStep === 0 || index === points.length - 1}
              <text
                class="statistics-chart-label"
                x={pointX(index, points.length, chartWidth)}
                y="166"
                text-anchor="middle">{axisLabel(point.date)}</text
              >
            {/if}
          {/each}
          {#if activePoint && activeIndex !== null}
            {@const x = pointX(activeIndex, points.length, chartWidth)}
            {@const dailyY =
              plotBottom - (activePoint.playtime_seconds / maximumDaily) * plotHeight}
            {@const cumulativeY =
              plotBottom - (activePoint.cumulative_seconds / maximumCumulative) * plotHeight}
            <line class="statistics-chart-cursor" x1={x} y1={plotTop} x2={x} y2={plotBottom} />
            <circle class="statistics-chart-daily-point" cx={x} cy={dailyY} r="4" />
            <circle class="statistics-chart-cumulative-point" cx={x} cy={cumulativeY} r="4" />
          {/if}
        </svg>
      </div>
      {#if activePoint}
        <div
          bind:this={tooltipElement}
          class:pinned={pinnedIndex !== null}
          class:align-start={tooltipPosition < 25}
          class:align-end={tooltipPosition > 75}
          class="statistics-chart-tooltip"
          style={`left: ${tooltipPosition}%`}
          aria-live="polite"
        >
          <strong>{formatDateKey(activePoint.date)}</strong>
          <dl>
            <div>
              <dt>合計</dt>
              <dd>{compactDuration(activePoint.playtime_seconds)}</dd>
            </div>
            <div>
              <dt>累計</dt>
              <dd>{compactDuration(activePoint.cumulative_seconds)}</dd>
            </div>
          </dl>
          {#if activeGames.length}
            <ul>
              {#each activeGames as game}<li style={`--game-color: ${gameColor(game.game_id)}`}>
                  <span class="statistics-chart-game">
                    <span class="statistics-chart-game-image">
                      {#if game.thumbnail_path}<img
                          src={imageSrc(game.thumbnail_path)}
                          alt=""
                        />{:else}<i></i>{/if}
                    </span>
                    <span class="statistics-chart-game-title">{game.title}</span>
                  </span>
                  <strong>{compactDuration(game.playtime_seconds)}</strong>
                </li>{/each}
            </ul>
          {:else}
            <p>プレイ記録なし</p>
          {/if}
        </div>
      {/if}
    </div>
  </div>
  {#if zoom > 1.01}
    <span class="statistics-chart-edge-fade start" aria-hidden="true"></span>
    <span class="statistics-chart-edge-fade end" aria-hidden="true"></span>
    <span class="statistics-chart-zoom" aria-live="polite">{Math.round(zoom * 100)}%</span>
  {/if}
</div>
