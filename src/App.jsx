import { useMemo, useState } from "react";
import "./App.css";

const splitOptions = [1, 2, 4];

function App() {
  const [split, setSplit] = useState(2);
  const [activePane, setActivePane] = useState(0);
  const [paneState, setPaneState] = useState(() =>
    Array.from({ length: 4 }, (_, idx) => ({
      id: idx,
      pair: "USD/JPY",
      timeframe: "M1",
      indicator: "MA",
    }))
  );

  const panes = useMemo(() => paneState.slice(0, split), [paneState, split]);

  const updatePane = (idx, patch) => {
    setPaneState((prev) =>
      prev.map((p, i) => (i === idx ? { ...p, ...patch } : p))
    );
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
          <button type="button" className="ghost">
            再生
          </button>
          <button type="button" className="ghost">
            速度 1x
          </button>
        </div>
      </header>

      <div className="layout">
        <aside className="sidebar">
          <div className="panel-title">通貨ペア</div>
          {["USD/JPY", "EUR/USD", "GBP/JPY", "AUD/USD"].map((pair) => (
            <button
              key={pair}
              type="button"
              className={
                paneState[activePane].pair === pair ? "list-item active" : "list-item"
              }
              onClick={() => updatePane(activePane, { pair })}
            >
              {pair}
            </button>
          ))}
        </aside>

        <main
          className={`chart-area split-${split}`}
          style={{ gridTemplateColumns: split === 1 ? "1fr" : split === 2 ? "1fr 1fr" : "1fr 1fr" }}
        >
          {panes.map((pane, idx) => (
            <section
              key={pane.id}
              className={
                idx === activePane ? "chart-pane active" : "chart-pane"
              }
              onClick={() => setActivePane(idx)}
            >
              <div className="pane-header">
                <span>{pane.pair}</span>
                <span>{pane.timeframe}</span>
              </div>
              <div className="pane-body">
                <div className="chart-placeholder">Chart</div>
              </div>
              <div className="pane-footer">{pane.indicator}</div>
            </section>
          ))}
        </main>

        <aside className="settings">
          <div className="panel-title">設定</div>
          <div className="setting-block">
            <label>足の種類</label>
            <div className="segmented">
              {["M1", "M5", "H1", "D1"].map((tf) => (
                <button
                  key={tf}
                  type="button"
                  className={paneState[activePane].timeframe === tf ? "active" : ""}
                  onClick={() => updatePane(activePane, { timeframe: tf })}
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
                  className={paneState[activePane].indicator === ind ? "active" : ""}
                  onClick={() => updatePane(activePane, { indicator: ind })}
                >
                  {ind}
                </button>
              ))}
            </div>
          </div>
        </aside>
      </div>
    </div>
  );
}

export default App;
