import { useEffect, useMemo, useState } from "react";
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
  indicator: "MA",
  indicatorData: null,
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
  const [rawDataset, setRawDataset] = useState(null);
  const [candles, setCandles] = useState([]);
  const [viewBars, setViewBars] = useState(240);
  const [viewOffset, setViewOffset] = useState(0);

  const panes = useMemo(() => paneState.slice(0, split), [paneState, split]);
  const active = paneState[activePane];

  const updatePane = (idx, patch) => {
    setPaneState((prev) =>
      prev.map((p, i) => (i === idx ? { ...p, ...patch } : p))
    );
  };

  const maxBars = candles.length || 0;

  const updateSeek = (nextSeek) => {
    const clamped = clamp(nextSeek, 0, Math.max(0, maxBars - 1));
    updatePane(activePane, { seek: clamped });
  };

  const syncViewToSeek = (seekValue, bars = viewBars) => {
    if (!maxBars) return;
    const maxOffset = Math.max(0, maxBars - bars);
    const nextOffset = clamp(seekValue - bars + 1, 0, maxOffset);
    setViewOffset(nextOffset);
  };

  useEffect(() => {
    if (!active.playing) return undefined;
    if (!maxBars) return undefined;

    const interval = Math.max(50, basePlaybackMs / active.speed);
    const id = window.setInterval(() => {
      setPaneState((prev) =>
        prev.map((p, idx) => {
          if (idx !== activePane) return p;
          const next = Math.min(p.seek + 1, Math.max(0, maxBars - 1));
          if (next === p.seek) {
            return { ...p, playing: false };
          }
          return { ...p, seek: next };
        })
      );
    }, interval);

    return () => window.clearInterval(id);
  }, [active.playing, active.speed, activePane, maxBars]);

  useEffect(() => {
    if (!maxBars) return;
    syncViewToSeek(active.seek, viewBars);
  }, [active.seek, viewBars, maxBars]);

  const refreshPresets = async () => {
    const list = await invoke("list_presets");
    setPresets(list);
  };

  const savePreset = async () => {
    if (!presetName.trim()) return;
    await invoke("save_preset", {
      preset: {
        name: presetName,
        split,
        panes: paneState,
      },
    });
    await refreshPresets();
    setPresetName("");
  };

  const loadPreset = async (name) => {
    const preset = await invoke("load_preset", { name });
    setSplit(preset.split);
    setPaneState(preset.panes);
    setActivePane(0);
  };

  const deletePreset = async (name) => {
    await invoke("delete_preset", { name });
    await refreshPresets();
  };

  const ingestCsv = async () => {
    setIngestError("");
    setIngestLoading(true);
    try {
      const file = await open({
        multiple: false,
        filters: [{ name: "CSV/TSV", extensions: ["csv", "tsv"] }],
      });
      if (!file) {
        setIngestLoading(false);
        return;
      }
      const result = await invoke("ingest_csv", { path: file });
      setIngestInfo({
        path: result.dataset.source_path,
        rows: result.dataset.candles.length,
        usedCache: result.used_cache,
      });
      setRawDataset(result.dataset);
      setCandles(result.dataset.candles);
      setViewBars(Math.min(240, result.dataset.candles.length || 240));
      setViewOffset(0);
      updatePane(activePane, { bars: result.dataset.candles.length, seek: 0 });
      const indicators = await invoke("compute_indicators", { dataset: result.dataset });
      updatePane(activePane, { indicatorData: indicators });
    } catch (err) {
      setIngestError(String(err));
    } finally {
      setIngestLoading(false);
    }
  };

  const applyTimeframe = async (nextTf) => {
    updatePane(activePane, { timeframe: nextTf });
    const dataset = rawDataset || (candles.length ? { source_path: "", candles } : null);
    if (!dataset) return;
    try {
      const resampled = await invoke("resample_dataset", {
        dataset,
        target: nextTf,
      });
      setCandles(resampled.candles);
      setViewBars(Math.min(240, resampled.candles.length || 240));
      setViewOffset(0);
      updatePane(activePane, { bars: resampled.candles.length, seek: 0 });
      const indicators = await invoke("compute_indicators", { dataset: resampled });
      updatePane(activePane, { indicatorData: indicators });
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
          {ingestError ? <div className="ingest-error">{ingestError}</div> : null}
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
                <span>{pane.timeframe}</span>
              </div>
              <div className="pane-body">
                <ChartCanvas
                  candles={candles}
                  viewBars={viewBars}
                  viewOffset={viewOffset}
                  onViewChange={(next) => {
                    if (next.viewBars !== undefined) setViewBars(next.viewBars);
                    if (next.viewOffset !== undefined) {
                      setViewOffset(next.viewOffset);
                      if (candles.length > 0) {
                        const nextSeek = clamp(
                          next.viewOffset + viewBars - 1,
                          0,
                          Math.max(0, candles.length - 1)
                        );
                        updatePane(activePane, { seek: nextSeek });
                      }
                    }
                  }}
                />
                {!candles || candles.length === 0 ? (
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
            <label>シーク</label>
            <input
              type="range"
              min="0"
              max={Math.max(0, active.bars - 1)}
              value={active.seek}
              onChange={(e) =>
                updateSeek(Number(e.target.value))
              }
            />
            <div className="seek-actions">
              <button type="button" className="ghost" onClick={() => updateSeek(active.seek - 1)}>
                -1
              </button>
              <button type="button" className="ghost" onClick={() => updateSeek(active.seek + 1)}>
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
