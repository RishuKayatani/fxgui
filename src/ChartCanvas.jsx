import { useEffect, useRef, useState } from "react";

const clamp = (value, min, max) => Math.max(min, Math.min(max, value));

const seriesValue = (value) => (value === null || value === undefined ? null : Number(value));

const drawLineSeries = (ctx, series, { startX, startY, chartWidth, chartHeight, bars, min, max }, color) => {
  const scale = chartHeight / (max - min);
  ctx.strokeStyle = color;
  ctx.lineWidth = 1.2;
  ctx.beginPath();
  let started = false;
  for (let i = 0; i < series.length; i += 1) {
    const value = seriesValue(series[i]);
    if (value === null || Number.isNaN(value)) {
      started = false;
      continue;
    }
    const x = startX + (chartWidth / bars) * i + (chartWidth / bars) / 2;
    const y = startY + (max - value) * scale;
    if (!started) {
      ctx.moveTo(x, y);
      started = true;
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();
};

const drawLineChart = (ctx, data, { startX, startY, chartWidth, chartHeight, bars, min, max }) => {
  const scale = chartHeight / (max - min);
  ctx.strokeStyle = "#5ad1ff";
  ctx.lineWidth = 1.4;
  ctx.beginPath();
  data.forEach((c, i) => {
    const x = startX + (chartWidth / bars) * i + (chartWidth / bars) / 2;
    const y = startY + (max - c.close) * scale;
    if (i === 0) ctx.moveTo(x, y);
    else ctx.lineTo(x, y);
  });
  ctx.stroke();
};

const drawBarChart = (ctx, data, { startX, startY, chartWidth, chartHeight, bars, min, max }) => {
  const scale = chartHeight / (max - min);
  const barWidth = Math.max(1, chartWidth / bars * 0.6);
  ctx.strokeStyle = "#ffd166";
  data.forEach((c, i) => {
    const x = startX + (chartWidth / bars) * i + (chartWidth / bars - barWidth) / 2;
    const highY = startY + (max - c.high) * scale;
    const lowY = startY + (max - c.low) * scale;
    ctx.beginPath();
    ctx.moveTo(x + barWidth / 2, highY);
    ctx.lineTo(x + barWidth / 2, lowY);
    ctx.stroke();
  });
};

export default function ChartCanvas({
  candles,
  viewBars,
  viewOffset,
  onViewChange,
  indicatorData,
  indicatorType,
  chartType,
  totalBars,
}) {
  const canvasRef = useRef(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const [crosshair, setCrosshair] = useState(null);
  const [dragStart, setDragStart] = useState(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return undefined;

    const observer = new ResizeObserver((entries) => {
      const rect = entries[0].contentRect;
      setSize({ width: rect.width, height: rect.height });
    });
    observer.observe(canvas.parentElement);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const t0 = import.meta.env.DEV ? performance.now() : 0;
    const canvas = canvasRef.current;
    if (!canvas) return;
    const ctx = canvas.getContext("2d");
    const dpr = window.devicePixelRatio || 1;
    const width = Math.max(1, Math.floor(size.width));
    const height = Math.max(1, Math.floor(size.height));
    canvas.width = width * dpr;
    canvas.height = height * dpr;
    canvas.style.width = `${width}px`;
    canvas.style.height = `${height}px`;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = "#0d1016";
    ctx.fillRect(0, 0, width, height);

    if (!candles || candles.length === 0) {
    ctx.fillStyle = "#6b7686";
    ctx.font = "12px 'JetBrains Mono', monospace";
      ctx.fillText("No data", 12, 20);
      return;
    }

    const total = Math.max(1, totalBars || candles.length);
    const minBars = Math.min(20, total);
    const bars = clamp(viewBars, minBars, total);
    const maxOffset = Math.max(0, total - bars);
    const offset = clamp(viewOffset, 0, maxOffset);
    const windowData = candles;

    let min = Infinity;
    let max = -Infinity;
    for (const c of windowData) {
      min = Math.min(min, c.low);
      max = Math.max(max, c.high);
    }
    const padding = (max - min) * 0.05 || 1;
    min -= padding;
    max += padding;

    const chartHeight = height - 24;
    const chartWidth = width - 24;
    const startX = 12;
    const startY = 12;
    const drawType = chartType || "Candlestick";
    if (drawType === "Line") {
      drawLineChart(ctx, windowData, { startX, startY, chartWidth, chartHeight, bars, min, max });
    } else if (drawType === "Bar") {
      drawBarChart(ctx, windowData, { startX, startY, chartWidth, chartHeight, bars, min, max });
    } else {
      const candleWidth = Math.max(2, chartWidth / bars * 0.6);
      windowData.forEach((c, i) => {
        const x = startX + (chartWidth / bars) * i + (chartWidth / bars - candleWidth) / 2;
        const scale = chartHeight / (max - min);
        const openY = startY + (max - c.open) * scale;
        const closeY = startY + (max - c.close) * scale;
        const highY = startY + (max - c.high) * scale;
        const lowY = startY + (max - c.low) * scale;
        const color = c.close >= c.open ? "#5ad1ff" : "#ff6b6b";
        ctx.strokeStyle = color;
        ctx.fillStyle = color;

        ctx.beginPath();
        ctx.moveTo(x + candleWidth / 2, highY);
        ctx.lineTo(x + candleWidth / 2, lowY);
        ctx.stroke();

        const bodyTop = Math.min(openY, closeY);
        const bodyHeight = Math.max(2, Math.abs(openY - closeY));
        ctx.fillRect(x, bodyTop, candleWidth, bodyHeight);
      });
    }

    if (indicatorData) {
      const key = indicatorType ? indicatorType.toLowerCase() : "ma";
      const series = Array.isArray(indicatorData)
        ? indicatorData
        : indicatorData[key] || indicatorData.ma;
      if (Array.isArray(series)) {
        const indicatorSlice = Array.isArray(indicatorData) ? series : series.slice(offset, offset + bars);
        let iMin = Infinity;
        let iMax = -Infinity;
        for (const v of indicatorSlice) {
          const value = seriesValue(v);
          if (value === null || Number.isNaN(value)) continue;
          iMin = Math.min(iMin, value);
          iMax = Math.max(iMax, value);
        }
        if (iMin !== Infinity && iMax !== -Infinity) {
          const padding = (iMax - iMin) * 0.05 || 1;
          iMin -= padding;
          iMax += padding;
          drawLineSeries(
            ctx,
            indicatorSlice,
            { startX, startY, chartWidth, chartHeight, bars, min: iMin, max: iMax },
            "#ffbe5a"
          );
        }
      }
    }

    if (crosshair) {
      ctx.strokeStyle = "rgba(255,255,255,0.2)";
      ctx.beginPath();
      ctx.moveTo(crosshair.x, startY);
      ctx.lineTo(crosshair.x, startY + chartHeight);
      ctx.moveTo(startX, crosshair.y);
      ctx.lineTo(startX + chartWidth, crosshair.y);
      ctx.stroke();
    }
    if (import.meta.env.DEV) {
      const duration = performance.now() - t0;
      console.info(`[perf] render.chart: ${duration.toFixed(1)}ms`);
    }
  }, [candles, size, viewBars, viewOffset, crosshair, indicatorData, indicatorType, chartType]);

  const handleWheel = (event) => {
    event.preventDefault();
    const delta = Math.sign(event.deltaY);
    const maxBars = totalBars || (candles && candles.length) || viewBars || 20;
    const minBars = Math.min(20, maxBars);
    const nextBars = clamp(viewBars + delta * 10, minBars, maxBars);
    onViewChange({ viewBars: nextBars });
  };

  const handleMouseDown = (event) => {
    setDragStart({ x: event.clientX, offset: viewOffset });
  };

  const handleMouseMove = (event) => {
    const rect = event.currentTarget.getBoundingClientRect();
    setCrosshair({
      x: clamp(event.clientX - rect.left, 0, rect.width),
      y: clamp(event.clientY - rect.top, 0, rect.height),
    });

    if (dragStart && (totalBars || (candles && candles.length > 0))) {
      const dx = event.clientX - dragStart.x;
      const shift = Math.round(dx / 6);
      const maxOffset = Math.max(0, (totalBars || candles.length) - viewBars);
      const nextOffset = clamp(dragStart.offset - shift, 0, maxOffset);
      onViewChange({ viewOffset: nextOffset });
    }
  };

  const handleMouseUp = () => setDragStart(null);

  return (
    <canvas
      ref={canvasRef}
      className="chart-canvas"
      onWheel={handleWheel}
      onMouseDown={handleMouseDown}
      onMouseMove={handleMouseMove}
      onMouseUp={handleMouseUp}
      onMouseLeave={() => {
        setCrosshair(null);
        setDragStart(null);
      }}
    />
  );
}
