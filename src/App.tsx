import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Chat from "./Chat";
import Settings from "./Settings";
import { Minus, Square, X, Settings as SettingsIcon, MessageSquare } from "lucide-react";
import "./App.css";

export type ThemeMode = "light" | "dark";

function App() {
  const [view, setView] = useState<"chat" | "settings">("chat");
  const [theme, setTheme] = useState<ThemeMode>("light");

  const appWindow = getCurrentWindow();

  useEffect(() => {
    const savedTheme = window.localStorage.getItem("ga-theme-mode");
    if (savedTheme === "light" || savedTheme === "dark") {
      setTheme(savedTheme);
    }
  }, []);

  useEffect(() => {
    document.documentElement.dataset.theme = theme;
    window.localStorage.setItem("ga-theme-mode", theme);
  }, [theme]);

  const toggleTheme = () => {
    setTheme((current) => (current === "light" ? "dark" : "light"));
  };

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[var(--app-bg)] text-[var(--ink)] font-sans selection:bg-[var(--ink)] selection:text-[var(--paper-bg)] p-2 sm:p-4">
      {/* 隐藏的 SVG 滤镜 - 降低了震荡频率，避免破坏布局 */}
      <svg className="hidden">
        <filter id="wobbly-edge">
          <feTurbulence type="fractalNoise" baseFrequency="0.005" numOctaves="2" result="noise" />
          <feDisplacementMap in="SourceGraphic" in2="noise" scale="1.5" xChannelSelector="R" yChannelSelector="G" />
        </filter>
      </svg>

      {/* 外层包裹容器：带粗野主义黑边框和物理阴影 */}
      <div 
        className="flex flex-col h-full w-full bg-[var(--chrome-bg)] border-[3px] border-[var(--line)] theme-card-shadow relative z-10 overflow-hidden" 
        style={{ borderRadius: '4px 12px 6px 16px / 16px 4px 12px 6px', color: "var(--ink)" }}
      >
        
        {/* 自定义可拖拽标题栏 (Windows 兼容) */}
        <div 
          data-tauri-drag-region 
          className="flex items-center justify-between h-12 bg-[var(--accent)] border-b-[3px] border-[var(--line)] shrink-0 px-3 cursor-move select-none"
        >
          {/* 左侧：应用标题/Logo区 */}
          <div data-tauri-drag-region className="flex items-center gap-2 pointer-events-none">
            <div
              className="w-6 h-6 border-[2px] border-[var(--line)] flex items-center justify-center"
              style={{
                borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px',
                backgroundColor: "var(--surface-bg)",
                boxShadow: "2px 2px 0 0 var(--shadow-strong)"
              }}
            >
              <div className="w-3 h-3 rounded-sm" style={{ backgroundColor: "var(--ink)" }}></div>
            </div>
            <span className="font-black tracking-tighter uppercase text-sm" style={{ fontFamily: '"Space Grotesk", system-ui, sans-serif', color: "var(--ink)" }}>
              GENERIC<span style={{ color: "var(--paper-bg)", textShadow: '0 0 0 var(--line), -1px -1px 0 var(--line), 1px -1px 0 var(--line), -1px 1px 0 var(--line), 1px 1px 0 var(--line)' }}>AGENT</span>
            </span>
          </div>

          {/* 右侧：窗口控制按钮 */}
          <div className="flex items-center gap-2">
            <button 
              onClick={() => appWindow.minimize()}
              className="w-7 h-7 flex items-center justify-center border-[2px] border-[var(--line)] hover:brightness-110 hover:translate-x-[1px] hover:translate-y-[1px] transition-all"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', backgroundColor: "var(--surface-bg)", boxShadow: "2px 2px 0 0 var(--shadow-strong)" }}
            >
              <Minus className="w-4 h-4" style={{ color: "var(--ink)" }} />
            </button>
            <button 
              onClick={() => appWindow.toggleMaximize()}
              className="w-7 h-7 flex items-center justify-center border-[2px] border-[var(--line)] hover:brightness-110 hover:translate-x-[1px] hover:translate-y-[1px] transition-all"
              style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px', backgroundColor: "var(--surface-bg)", boxShadow: "2px 2px 0 0 var(--shadow-strong)" }}
            >
              <Square className="w-3 h-3" style={{ color: "var(--ink)" }} />
            </button>
            <button 
              onClick={() => appWindow.close()}
              className="w-7 h-7 flex items-center justify-center border-[2px] border-[var(--line)] hover:brightness-110 hover:translate-x-[1px] hover:translate-y-[1px] transition-all"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', backgroundColor: "var(--danger)", boxShadow: "2px 2px 0 0 var(--shadow-strong)" }}
            >
              <X className="w-4 h-4" style={{ color: "var(--ink)" }} />
            </button>
          </div>
        </div>

        {/* 内部主体：侧边栏 + 内容区 */}
        <div className="flex flex-1 overflow-hidden bg-[var(--chrome-bg)]">
          {/* 重构的侧边栏导航：紧凑、清晰的黑白分割 */}
          <div className="w-16 sm:w-20 bg-[var(--sidebar-bg)] flex flex-col items-center py-6 gap-6 shrink-0 border-r-[3px] border-[var(--line)] z-20 relative">
            <button
              onClick={() => setView("chat")}
              className={`p-2 sm:p-3 transition-all border-[3px] border-[var(--line)] relative group flex items-center justify-center w-10 h-10 sm:w-12 sm:h-12 hover:brightness-110 ${
                view === "chat" 
                  ? "translate-x-[2px] translate-y-[2px]" 
                  : ""
              }`}
              title="Chat"
              style={{
                borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px',
                backgroundColor: view === "chat" ? "var(--ink)" : "var(--surface-bg)",
                boxShadow: view === "chat" ? "2px 2px 0 0 var(--shadow-strong)" : "4px 4px 0 0 var(--shadow-strong)",
                color: view === "chat" ? "var(--paper-bg)" : "var(--ink)"
              }}
            >
              <MessageSquare className="w-5 h-5 sm:w-6 sm:h-6" style={{ color: view === "chat" ? "var(--paper-bg)" : "var(--ink)" }} />
            </button>
            
            <button
              onClick={() => setView("settings")}
              className={`p-2 sm:p-3 transition-all border-[3px] border-[var(--line)] relative group flex items-center justify-center w-10 h-10 sm:w-12 sm:h-12 hover:brightness-110 ${
                view === "settings" 
                  ? "translate-x-[2px] translate-y-[2px]" 
                  : ""
              }`}
              title="Settings"
              style={{
                borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px',
                backgroundColor: view === "settings" ? "var(--ink)" : "var(--surface-bg)",
                boxShadow: view === "settings" ? "2px 2px 0 0 var(--shadow-strong)" : "4px 4px 0 0 var(--shadow-strong)",
                color: view === "settings" ? "var(--paper-bg)" : "var(--ink)"
              }}
            >
              <SettingsIcon className="w-5 h-5 sm:w-6 sm:h-6" style={{ color: view === "settings" ? "var(--paper-bg)" : "var(--ink)" }} />
            </button>
            
            <div className="flex-1"></div>
            
            {/* 底部装饰标志 */}
            <div className="writing-vertical-rl font-black tracking-widest text-[10px] sm:text-xs uppercase pointer-events-none mb-4" style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)', color: "var(--ink-soft)" }}>
              GA-ENGINE
            </div>
          </div>

          {/* 主内容区域：纸张质感 */}
          <div className="flex-1 h-full relative bg-[var(--paper-bg)] overflow-hidden">
            {view === "chat" ? <Chat /> : <Settings theme={theme} onToggleTheme={toggleTheme} />}
          </div>
        </div>

      </div>
    </div>
  );
}

export default App;
