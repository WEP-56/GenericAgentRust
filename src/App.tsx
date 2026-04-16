import { useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import Chat from "./Chat";
import Settings from "./Settings";
import { Minus, Square, X, Settings as SettingsIcon, MessageSquare } from "lucide-react";
import "./App.css";

function App() {
  const [view, setView] = useState<"chat" | "settings">("chat");

  const appWindow = getCurrentWindow();

  return (
    <div className="flex flex-col h-screen w-screen overflow-hidden bg-[#e0e5ec] font-sans selection:bg-black selection:text-white p-2 sm:p-4">
      {/* 隐藏的 SVG 滤镜 - 降低了震荡频率，避免破坏布局 */}
      <svg className="hidden">
        <filter id="wobbly-edge">
          <feTurbulence type="fractalNoise" baseFrequency="0.005" numOctaves="2" result="noise" />
          <feDisplacementMap in="SourceGraphic" in2="noise" scale="1.5" xChannelSelector="R" yChannelSelector="G" />
        </filter>
      </svg>

      {/* 外层包裹容器：带粗野主义黑边框和物理阴影 */}
      <div 
        className="flex flex-col h-full w-full bg-white border-[3px] border-black shadow-[6px_6px_0px_0px_rgba(0,0,0,1)] relative z-10 overflow-hidden" 
        style={{ borderRadius: '4px 12px 6px 16px / 16px 4px 12px 6px' }}
      >
        
        {/* 自定义可拖拽标题栏 (Windows 兼容) */}
        <div 
          data-tauri-drag-region 
          className="flex items-center justify-between h-12 bg-[#ffde59] border-b-[3px] border-black shrink-0 px-3 cursor-move select-none"
        >
          {/* 左侧：应用标题/Logo区 */}
          <div data-tauri-drag-region className="flex items-center gap-2 pointer-events-none">
            <div className="w-6 h-6 bg-white border-[2px] border-black flex items-center justify-center shadow-[2px_2px_0px_0px_rgba(0,0,0,1)]" style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px' }}>
              <div className="w-3 h-3 bg-black rounded-sm"></div>
            </div>
            <span className="font-black text-black tracking-tighter uppercase text-sm" style={{ fontFamily: '"Space Grotesk", system-ui, sans-serif' }}>
              GENERIC<span className="text-white" style={{ textShadow: '-1px -1px 0 #000, 1px -1px 0 #000, -1px 1px 0 #000, 1px 1px 0 #000' }}>AGENT</span>
            </span>
          </div>

          {/* 右侧：窗口控制按钮 */}
          <div className="flex items-center gap-2">
            <button 
              onClick={() => appWindow.minimize()}
              className="w-7 h-7 flex items-center justify-center bg-white border-[2px] border-black shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] hover:bg-[#e0e5ec] hover:translate-x-[1px] hover:translate-y-[1px] hover:shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] transition-all"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px' }}
            >
              <Minus className="w-4 h-4 text-black" />
            </button>
            <button 
              onClick={() => appWindow.toggleMaximize()}
              className="w-7 h-7 flex items-center justify-center bg-white border-[2px] border-black shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] hover:bg-[#e0e5ec] hover:translate-x-[1px] hover:translate-y-[1px] hover:shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] transition-all"
              style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px' }}
            >
              <Square className="w-3 h-3 text-black" />
            </button>
            <button 
              onClick={() => appWindow.close()}
              className="w-7 h-7 flex items-center justify-center bg-[#ff9999] border-[2px] border-black shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] hover:bg-red-500 hover:text-white hover:translate-x-[1px] hover:translate-y-[1px] hover:shadow-[1px_1px_0px_0px_rgba(0,0,0,1)] transition-all"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px' }}
            >
              <X className="w-4 h-4 text-black" />
            </button>
          </div>
        </div>

        {/* 内部主体：侧边栏 + 内容区 */}
        <div className="flex flex-1 overflow-hidden bg-white">
          {/* 重构的侧边栏导航：紧凑、清晰的黑白分割 */}
          <div className="w-16 sm:w-20 bg-white flex flex-col items-center py-6 gap-6 shrink-0 border-r-[3px] border-black z-20 relative">
            <button
              onClick={() => setView("chat")}
              className={`p-2 sm:p-3 transition-all border-[3px] border-black relative group flex items-center justify-center w-10 h-10 sm:w-12 sm:h-12 ${
                view === "chat" 
                  ? "bg-black text-white shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] translate-x-[2px] translate-y-[2px]" 
                  : "bg-white text-black hover:bg-[#ffde59] shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]"
              }`}
              title="Chat"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px' }}
            >
              <MessageSquare className={`w-5 h-5 sm:w-6 sm:h-6 ${view === "chat" ? "text-white" : "text-black"}`} />
            </button>
            
            <button
              onClick={() => setView("settings")}
              className={`p-2 sm:p-3 transition-all border-[3px] border-black relative group flex items-center justify-center w-10 h-10 sm:w-12 sm:h-12 ${
                view === "settings" 
                  ? "bg-black text-white shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] translate-x-[2px] translate-y-[2px]" 
                  : "bg-white text-black hover:bg-[#ffde59] shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]"
              }`}
              title="Settings"
              style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px' }}
            >
              <SettingsIcon className={`w-5 h-5 sm:w-6 sm:h-6 ${view === "settings" ? "text-white" : "text-black"}`} />
            </button>
            
            <div className="flex-1"></div>
            
            {/* 底部装饰标志 */}
            <div className="writing-vertical-rl text-black font-black tracking-widest text-[10px] sm:text-xs opacity-30 uppercase pointer-events-none mb-4" style={{ writingMode: 'vertical-rl', transform: 'rotate(180deg)' }}>
              GA-ENGINE
            </div>
          </div>

          {/* 主内容区域：纸张质感 */}
          <div className="flex-1 h-full relative bg-[#fdfbf7] overflow-hidden">
            {view === "chat" ? <Chat /> : <Settings />}
          </div>
        </div>

      </div>
    </div>
  );
}

export default App;
