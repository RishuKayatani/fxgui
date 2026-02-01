import { useCallback, useEffect, useMemo, useState } from "react";
import { open } from "@tauri-apps/api/dialog";
import { invoke } from "@tauri-apps/api/core";
import ChartCanvas from "./ChartCanvas";
import "./App.css";

const splitOptions = [1, 2, 4];
const speedOptions = [0.5, 1, 2, 5, 10];
const basePlaybackMs = 500;
const clamp = (value, min, max) => Math.max(min, Math.min(max, value));

const emptyPane = (idx) => ({
  id: idx,
  pair: "USD/JPY",
  timeframe: "M1",
  chartType: "Candlestick",
  indicator: "MA",
  indicatorData: null,
  rawDataset: null,
  candles: [],
  viewBars: 240,
  viewOffset: 0,
  playing: false,
  speed: 1,
  seek: 0,
  bars: 240,
});

function App() {
  const [split, setSplit] = useState(2);
  const [activePane, setActivePane] = useState(0);
  const [paneState, setPaneState] = useState(() =>
    Array.from({ length: 4 }, (_, idx) => emptyPane(idx))
  );
  const [presets, setPresets] = useState([]);
  const [presetName, setPresetName] = useState("");
  const [ingestInfo, setIngestInfo] = useState(null);
  const [ingestError, setIngestError] = useState("");
  const [ingestLoading, setIngestLoading] = useState(false);
  const [syncEnabled, setSyncEnabled] = useState(false);
  const [perfWarning, setPerfWarning] = useState("");
  const [cacheInfo, setCacheInfo] = useState(null);
  const [datasetHistory, setDatasetHistory] = useState([]);

  const panes = useMemo(() => paneState.slice(0, split), [paneState, split]);
  const active = paneState[activePane];

  const updatePane = (idx, patch) => {
    setPaneState((prev) =>
      prev.map((p, i) => (i === idx ? { ...p, ...patch } : p))
    );
  };

  const toggleSync = () => {
    setSyncEnabled((prev) => {
      if (!prev) {
        const anchor = paneState[activePane];
        setPaneState((current) =>
          current.map((p, idx) => {
            if (idx === activePane) return p;
            return {
              ...p,
              viewBars: anchor.viewBars,
              viewOffset: anchor.viewOffset,
              seek: anchor.seek,
            };
          })
        );
      }
      return !prev;
    });
  };

  const maxBars = active.candles.length || 0;

  const updateSeek = (nextSeek) => {
    setPaneState((prev) =>
      prev.map((p, idx) => {
        if (!syncEnabled && idx !== activePane) return p;
        const limit = Math.max(0, p.candles.length - 1);
        const clamped = clamp(nextSeek, 0, limit);
        return { ...p, seek: clamped };
      })
    );
  };

  const applySeek = useCallback((nextSeek) => {
    updateSeek(nextSeek);
    setPaneState((prev) =>
      prev.map((p, idx) => {
        if (!syncEnabled && idx !== activePane) return p;
        const maxOffset = Math.max(0, p.candles.length - p.viewBars);
        const clampedSeek = clamp(nextSeek, 0, Math.max(0, p.candles.length - 1));
        const nextOffset = clamp(clampedSeek - p.viewBars + 1, 0, maxOffset);
        return { ...p, viewOffset: nextOffset, seek: clampedSeek };
      })
    );
  }, [activePane, syncEnabled]);

  const syncViewToSeek = (seekValue, bars, pane) => {
    if (!pane.candles.length) return pane;
    const maxOffset = Math.max(0, pane.candles.length - bars);
    const nextOffset = clamp(seekValue - bars + 1, 0, maxOffset);
    return { ...pane, viewOffset: nextOffset };
  };

  useEffect(() => {
    if (!active.playing) return undefined;
    if (!maxBars) return undefined;

    const interval = Math.max(50, basePlaybackMs / active.speed);
    const id = window.setInterval(() => {
      setPaneState((prev) => {
        const nextSeek = Math.min(active.seek + 1, Math.max(0, maxBars - 1));
        return prev.map((p, idx) => {
          if (!syncEnabled && idx !== activePane) return p;
          if (idx === activePane && nextSeek === active.seek) {
            return { ...p, playing: false };
          }
          const limit = Math.max(0, p.candles.length - 1);
          return { ...p, seek: clamp(nextSeek, 0, limit) };
        });
      });
    }, interval);

    return () => window.clearInterval(id);
  }, [active.playing, active.speed, active.seek, activePane, maxBars, syncEnabled]);

  useEffect(() => {
    if (!maxBars) return;
    setPaneState((prev) =>
      prev.map((p, idx) => {
        if (!syncEnabled && idx !== activePane) return p;
        return syncViewToSeek(active.seek, p.viewBars, p);
      })
    );
  }, [active.seek, activePane, maxBars, syncEnabled]);

  useEffect(() => {
    const handler = (event) => {
      if (event.target && ["INPUT", "TEXTAREA"].includes(event.target.tagName)) {
        return;
      }
      if (event.code === "Space") {
        event.preventDefault();
        updatePane(activePane, { playing: !active.playing });
      }
      if (event.key === "1") {
        setSplit(1);
        setActivePane(0);
      }
      if (event.key === "2") {
        setSplit(2);
        setActivePane(0);
      }
      if (event.key === "4") {
        setSplit(4);
        setActivePane(0);
      }
      if (event.key === "ArrowLeft") {
        applySeek(active.seek - 1);
      }
      if (event.key === "ArrowRight") {
        applySeek(active.seek + 1);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [active.playing, active.seek, activePane, applySeek]);

  const refreshPresets = async () => {
    const list = await invoke("list_presets");
    setPresets(list);
  };

  const refreshCacheInfo = async () => {
    const info = await invoke("cache_status");
    setCacheInfo(info);
  };

  const refreshDatasetHistory = async () => {
    const history = await invoke("list_dataset_history");
    setDatasetHistory(history);
  };

  const clearCacheUi = async () => {
    await invoke("clear_cache");
    await refreshCacheInfo();
  };

  const savePreset = async () => {
    if (!presetName.trim()) return;
    await invoke("save_preset", {
      preset: {
        name: presetName,
        split,
        panes: paneState.map((pane) => ({
          id: pane.id,
          pair: pane.pair,
          timeframe: pane.timeframe,
          indicator: pane.indicator,
          view_bars: pane.viewBars,
          view_offset: pane.viewOffset,
          playing: false,
          speed: pane.speed,
          seek: pane.seek,
          bars: pane.bars,
        })),
      },
    });
    await refreshPresets();
    setPresetName("");
  };

  const loadPreset = async (name) => {
    const preset = await invoke("load_preset", { name });
    setSplit(preset.split);
    setPaneState(
      preset.panes.map((pane, idx) => ({
        ...emptyPane(idx),
        id: pane.id ?? idx,
        pair: pane.pair,
        timeframe: pane.timeframe,
        indicator: pane.indicator,
        viewBars: pane.view_bars ?? 240,
        viewOffset: pane.view_offset ?? 0,
        speed: pane.speed,
        seek: pane.seek,
        bars: pane.bars,
      }))
    );
    setActivePane(0);
  };

  const deletePreset = async (name) => {
    await invoke("delete_preset", { name });
    await refreshPresets();
  };

  const ingestCsv = async (overridePath) => {
    setIngestError("");
    setPerfWarning("");
    setIngestLoading(true);
    try {
      const file = overridePath
        || await open({
          multiple: false,
          filters: [{ name: "CSV/TSV", extensions: ["csv", "tsv"] }],
        });
      if (!file) {
        setIngestLoading(false);
        return;
      }
      const result = await invoke("ingest_csv", { path: file });
      await invoke("record_dataset_history", { path: file });
      setIngestInfo({
        path: result.dataset.source_path,
        rows: result.dataset.candles.length,
        usedCache: result.used_cache,
      });
      const nextBars = Math.min(240, result.dataset.candles.length || 240);
      const indicators = await invoke("compute_indicators", { dataset: result.dataset });
      updatePane(activePane, {
        rawDataset: result.dataset,
        candles: result.dataset.candles,
        viewBars: nextBars,
        viewOffset: 0,
        bars: result.dataset.candles.length,
        seek: 0,
        indicatorData: indicators,
      });
      if (result.dataset.candles.length >= 100000) {
        setPerfWarning("大量データ（10万バー以上）です。動作が重くなる可能性があります。");
      }
      await refreshCacheInfo();
      await refreshDatasetHistory();
    } catch (err) {
      const message = String(err || "読み込みに失敗しました").replace(/^Error:\s*/i, "");
      setIngestError(message);
    } finally {
      setIngestLoading(false);
    }
  };

  const applyTimeframe = async (nextTf) => {
    updatePane(activePane, { timeframe: nextTf });
    const dataset =
      active.rawDataset ||
      (active.candles.length ? { source_path: "", candles: active.candles } : null);
    if (!dataset) return;
    try {
      const resampled = await invoke("resample_dataset", {
        dataset,
        target: nextTf,
      });
      const indicators = await invoke("compute_indicators", { dataset: resampled });
      const nextBars = Math.min(240, resampled.candles.length || 240);
      updatePane(activePane, {
        candles: resampled.candles,
        viewBars: nextBars,
        viewOffset: 0,
        bars: resampled.candles.length,
        seek: 0,
        indicatorData: indicators,
      });
    } catch (err) {
      setIngestError(String(err));
    }
  };

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="topbar-left">
          <div className="brand">fxgui</div>
          <div className="split-toggle">
            {splitOptions.map((value) => (
              <button
                key={value}
                type="button"
                className={value === split ? "active" : ""}
                onClick={() => {
                  setSplit(value);
                  setActivePane(0);
                }}
              >
                {value}画面
              </button>
            ))}
          </div>
        </div>
        <div className="topbar-right">
          <button
            type="button"
            className={active.playing ? "ghost active" : "ghost"}
            onClick={() => updatePane(activePane, { playing: !active.playing })}
          >
            {active.playing ? "一時停止" : "再生"}
          </button>
          <button
            type="button"
            className={syncEnabled ? "ghost active" : "ghost"}
            onClick={toggleSync}
          >
            同期 {syncEnabled ? "ON" : "OFF"}
          </button>
          <div className="speed-group">
            {speedOptions.map((value) => (
              <button
                key={value}
                type="button"
                className={active.speed === value ? "ghost active" : "ghost"}
                onClick={() => updatePane(activePane, { speed: value })}
              >
                {value}x
              </button>
            ))}
          </div>
        </div>
      </header>

      <div className="layout">
        <aside className="sidebar">
          <div className="panel-title">通貨ペア</div>
          {[
            "USD/JPY",
            "EUR/USD",
            "GBP/JPY",
            "AUD/USD",
          ].map((pair) => (
            <button
              key={pair}
              type="button"
              className={
                paneState[activePane].pair === pair
                  ? "list-item active"
                  : "list-item"
              }
              onClick={() => updatePane(activePane, { pair })}
            >
              {pair}
            </button>
          ))}
          <div className="panel-title">CSV</div>
          <button
            type="button"
            className="ghost"
            onClick={ingestCsv}
            disabled={ingestLoading}
          >
            {ingestLoading ? "読み込み中..." : "CSV読み込み"}
          </button>
          {ingestInfo ? (
            <div className="ingest-info">
              <div>rows: {ingestInfo.rows}</div>
              <div>cache: {ingestInfo.usedCache ? "hit (cached)" : "miss (parsed)"}</div>
            </div>
          ) : null}
          {perfWarning ? <div className="perf-warning">{perfWarning}</div> : null}
          {ingestError ? (
            <div className="ingest-error">
              <div className="ingest-error-title">読み込みに失敗しました</div>
              <div className="ingest-error-body">{ingestError}</div>
              <button type="button" className="ghost" onClick={ingestCsv}>
                再読み込み
              </button>
            </div>
          ) : null}
          <div className="cache-panel">
            <div className="panel-title">Cache</div>
            <div className="cache-row">
              <button type="button" className="ghost" onClick={refreshCacheInfo}>
                情報更新
              </button>
              <button type="button" className="ghost" onClick={clearCacheUi}>
                キャッシュ削除
              </button>
            </div>
            {cacheInfo ? (
              <div className="cache-meta">
                <div>path: {cacheInfo.path}</div>
                <div>files: {cacheInfo.files}</div>
                <div>size: {Math.round(cacheInfo.bytes / 1024)} KB</div>
              </div>
            ) : (
              <div className="cache-meta">未取得</div>
            )}
          </div>
          <div className="cache-panel">
            <div className="panel-title">最近使ったCSV</div>
            <div className="cache-row">
              <button type="button" className="ghost" onClick={refreshDatasetHistory}>
                履歴更新
              </button>
            </div>
            {datasetHistory.length ? (
              <div className="history-list">
                {datasetHistory.map((item) => (
                  <button
                    key={item.path}
                    type="button"
                    className="list-item"
                    onClick={() => ingestCsv(item.path)}
                  >
                    {item.path}
                  </button>
                ))}
              </div>
            ) : (
              <div className="cache-meta">履歴なし</div>
            )}
          </div>
        </aside>

        <main
          className={`chart-area split-${split}`}
          style={{
            gridTemplateColumns:
              split === 1 ? "1fr" : split === 2 ? "1fr 1fr" : "1fr 1fr",
          }}
        >
          {panes.map((pane, idx) => (
            <section
              key={pane.id}
              className={idx === activePane ? "chart-pane active" : "chart-pane"}
              onClick={() => setActivePane(idx)}
            >
              <div className="pane-header">
                <span>{pane.pair}</span>
                <span>{pane.timeframe} UTC</span>
              </div>
              <div className="pane-body">
                <ChartCanvas
                  candles={pane.candles}
                  viewBars={pane.viewBars}
                  viewOffset={pane.viewOffset}
                  indicatorData={pane.indicatorData}
                  indicatorType={pane.indicator}
                  chartType={pane.chartType}
                  onViewChange={(next) => {
                    if (next.viewBars !== undefined) {
                      setPaneState((prev) =>
                        prev.map((p, pIdx) => {
                          if (!syncEnabled && pIdx !== idx) return p;
                          const limit = Math.max(20, p.candles.length || 20);
                          const clampedBars = clamp(next.viewBars, 20, limit);
                          const maxOffset = Math.max(0, p.candles.length - clampedBars);
                          const nextOffset = clamp(p.viewOffset, 0, maxOffset);
                          return { ...p, viewBars: clampedBars, viewOffset: nextOffset };
                        })
                      );
                    }
                    if (next.viewOffset !== undefined) {
                      setPaneState((prev) =>
                        prev.map((p, pIdx) => {
                          if (!syncEnabled && pIdx !== idx) return p;
                          const maxOffset = Math.max(0, p.candles.length - p.viewBars);
                          const nextOffset = clamp(next.viewOffset, 0, maxOffset);
                          const nextSeek = clamp(
                            nextOffset + p.viewBars - 1,
                            0,
                            Math.max(0, p.candles.length - 1)
                          );
                          return { ...p, viewOffset: nextOffset, seek: nextSeek };
                        })
                      );
                    }
                  }}
                />
                {!pane.candles || pane.candles.length === 0 ? (
                  <div className="chart-overlay">
                    <div>
                      <div>CSVを読み込むと表示されます</div>
                      <div>左の「CSV読み込み」から選択してください</div>
                    </div>
                  </div>
                ) : null}
              </div>
              <div className="pane-footer">
                {pane.indicator}
                {pane.indicatorData ? " · ready" : ""}
                <span className="seek-label">
                  {pane.seek} / {pane.bars}
                </span>
              </div>
            </section>
          ))}
        </main>

        <aside className="settings">
          <div className="panel-title">設定</div>
          <div className="setting-block">
            <label>足の種類</label>
            <div className="segmented">
              {["M1", "M5", "M15", "M30", "H1", "H4", "D1"].map((tf) => (
                <button
                  key={tf}
                  type="button"
                  className={active.timeframe === tf ? "active" : ""}
                  onClick={() => applyTimeframe(tf)}
                >
                  {tf}
                </button>
              ))}
            </div>
          </div>
          <div className="setting-block">
            <label>インジケーター</label>
            <div className="segmented">
              {["MA", "RSI", "MACD"].map((ind) => (
                <button
                  key={ind}
                  type="button"
                  className={active.indicator === ind ? "active" : ""}
                  onClick={() => updatePane(activePane, { indicator: ind })}
                >
                  {ind}
                </button>
              ))}
            </div>
          </div>
          <div className="setting-block">
            <label>チャート種別</label>
            <div className="segmented">
              {["Candlestick", "Line", "Bar"].map((mode) => (
                <button
                  key={mode}
                  type="button"
                  className={active.chartType === mode ? "active" : ""}
                  onClick={() => updatePane(activePane, { chartType: mode })}
                >
                  {mode}
                </button>
              ))}
            </div>
          </div>
          <div className="setting-block">
            <label>シーク</label>
            <input
              type="range"
              min="0"
              max={Math.max(0, active.bars - 1)}
              value={active.seek}
              onChange={(e) =>
                applySeek(Number(e.target.value))
              }
            />
            <div className="seek-actions">
              <button type="button" className="ghost" onClick={() => applySeek(active.seek - 1)}>
                -1
              </button>
              <button type="button" className="ghost" onClick={() => applySeek(active.seek + 1)}>
                +1
              </button>
            </div>
            <div className="seek-meta">{active.seek} / {active.bars} bars</div>
          </div>
          <div className="setting-block">
            <label>プリセット</label>
            <div className="preset-row">
              <input
                type="text"
                value={presetName}
                placeholder="preset name"
                onChange={(e) => setPresetName(e.target.value)}
              />
              <button type="button" className="ghost" onClick={savePreset}>
                保存
              </button>
              <button type="button" className="ghost" onClick={refreshPresets}>
                更新
              </button>
            </div>
            <div className="preset-list">
              {presets.map((p) => (
                <div key={p.name} className="preset-item">
                  <button type="button" onClick={() => loadPreset(p.name)}>
                    {p.name}
                  </button>
                  <button
                    type="button"
                    className="ghost"
                    onClick={() => deletePreset(p.name)}
                  >
                    削除
                  </button>
                </div>
              ))}
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

export default App;
