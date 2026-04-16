import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Save, Plus, Trash2, MoonStar, SunMedium } from "lucide-react";
import type { ThemeMode } from "./App";

interface LlmProvider {
  id: string;
  name: string;
  base_url: string;
  api_key: string;
  default_model: string;
  max_tokens: number;
  temperature: number;
  is_native_anthropic: boolean;
  max_retries: number;
}

interface AppConfig {
  providers: LlmProvider[];
  active_provider_id: string;
  workspace_dir: string;
  memory_dir: string;
}

interface SettingsProps {
  theme: ThemeMode;
  onToggleTheme: () => void;
}

export default function Settings({ theme, onToggleTheme }: SettingsProps) {
  const [config, setConfig] = useState<AppConfig | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<AppConfig>("get_app_config").then((cfg) => {
      setConfig(cfg);
      setLoading(false);
    });
  }, []);

  const saveConfig = async () => {
    if (!config) return;
    try {
      await invoke("save_app_config", { config });
      alert("Settings saved!");
    } catch (e) {
      alert("Failed to save: " + e);
    }
  };

  if (loading || !config) {
    return (
      <div className="p-8 h-full overflow-y-auto font-sans" style={{ color: "var(--ink)", backgroundColor: "var(--paper-bg)" }}>
        Loading settings...
      </div>
    );
  }

  return (
    <div className="p-6 md:p-8 h-full overflow-y-auto font-sans" style={{ backgroundColor: "var(--paper-bg)", color: "var(--ink)" }}>
      <div className="max-w-5xl mx-auto space-y-8 pb-12">
        <div
          className="border-[3px] px-5 py-5 md:px-6 md:py-6 flex flex-col gap-4 md:flex-row md:items-center md:justify-between"
          style={{
            backgroundColor: "var(--surface-bg)",
            borderColor: "var(--line)",
            boxShadow: "6px 6px 0 0 var(--shadow-strong)",
            borderRadius: "15px 255px 15px 225px / 225px 15px 255px 15px"
          }}
        >
          <div>
            <div
              className="inline-block px-3 py-1 text-xs font-black uppercase tracking-[0.3em] border-[2px] mb-3"
              style={{
                backgroundColor: "var(--accent)",
                borderColor: "var(--line)",
                boxShadow: "2px 2px 0 0 var(--shadow-strong)",
                transform: "rotate(-1deg)"
              }}
            >
              Control Deck
            </div>
            <h1 className="text-3xl md:text-4xl font-black uppercase tracking-tight">Agent Settings</h1>
            <p className="mt-2 text-sm md:text-base" style={{ color: "var(--ink-soft)" }}>
              配置 LLM、工作目录和主题。保持桌面端本地代理的最小配置面。
            </p>
          </div>

          <div className="flex flex-wrap gap-3">
            <button
              onClick={onToggleTheme}
              className="flex items-center gap-2 px-4 py-2.5 border-[3px] hover:brightness-110 transition-all"
              style={{
                backgroundColor: "var(--surface-muted)",
                color: "var(--ink)",
                borderColor: "var(--line)",
                boxShadow: "4px 4px 0 0 var(--shadow-strong)",
                borderRadius: "255px 15px 225px 15px / 15px 225px 15px 255px"
              }}
            >
              {theme === "light" ? <MoonStar className="w-4 h-4" /> : <SunMedium className="w-4 h-4" />}
              <span className="font-black uppercase tracking-wide">
                {theme === "light" ? "Dark Mode" : "Light Mode"}
              </span>
            </button>
            <button
              onClick={saveConfig}
              className="flex items-center gap-2 px-5 py-2.5 border-[3px] hover:brightness-110 transition-all"
              style={{
                backgroundColor: "var(--ink)",
                color: "var(--paper-bg)",
                borderColor: "var(--line)",
                boxShadow: "4px 4px 0 0 var(--shadow-strong)",
                borderRadius: "15px 255px 15px 225px / 225px 15px 255px 15px"
              }}
            >
              <Save className="w-4 h-4" />
              <span className="font-black uppercase tracking-wide">Save</span>
            </button>
          </div>
        </div>

        <section
          className="p-6 border-[3px]"
          style={{
            backgroundColor: "var(--surface-bg)",
            borderColor: "var(--line)",
            boxShadow: "6px 6px 0 0 var(--shadow-strong)",
            borderRadius: "18px 30px 14px 34px / 24px 12px 28px 16px"
          }}
        >
          <div className="flex items-center justify-between gap-4 mb-6">
            <div>
              <h2 className="text-xl font-black uppercase tracking-wide">General</h2>
              <p className="text-sm mt-1" style={{ color: "var(--ink-soft)" }}>
                指定工作区与 memory 目录，后端 runtime 会基于这里构造上下文。
              </p>
            </div>
            <div
              className="px-3 py-1 text-[11px] font-black uppercase border-[2px]"
              style={{
                backgroundColor: "var(--accent)",
                borderColor: "var(--line)",
                boxShadow: "2px 2px 0 0 var(--shadow-strong)",
                transform: "rotate(1deg)"
              }}
            >
              Core Paths
            </div>
          </div>
          <div className="grid gap-5">
            <div>
              <label className="block text-sm font-black uppercase tracking-wide mb-2">Workspace Directory</label>
              <input
                type="text"
                value={config.workspace_dir}
                onChange={(e) => setConfig({ ...config, workspace_dir: e.target.value })}
                className="w-full p-3 border-[3px] outline-none theme-input"
                style={{
                  backgroundColor: "var(--paper-bg)",
                  color: "var(--ink)",
                  borderColor: "var(--line)",
                  borderRadius: "14px 18px 12px 20px / 20px 12px 18px 14px",
                  boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                }}
              />
            </div>
            <div>
              <label className="block text-sm font-black uppercase tracking-wide mb-2">Memory Directory</label>
              <input
                type="text"
                value={config.memory_dir}
                onChange={(e) => setConfig({ ...config, memory_dir: e.target.value })}
                className="w-full p-3 border-[3px] outline-none theme-input"
                style={{
                  backgroundColor: "var(--paper-bg)",
                  color: "var(--ink)",
                  borderColor: "var(--line)",
                  borderRadius: "20px 12px 18px 14px / 14px 20px 12px 18px",
                  boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                }}
              />
            </div>
          </div>
        </section>

        <section
          className="p-6 border-[3px]"
          style={{
            backgroundColor: "var(--surface-bg)",
            borderColor: "var(--line)",
            boxShadow: "6px 6px 0 0 var(--shadow-strong)",
            borderRadius: "24px 12px 30px 16px / 14px 26px 18px 28px"
          }}
        >
          <div className="flex flex-col gap-4 md:flex-row md:items-center md:justify-between mb-6">
            <div>
              <h2 className="text-xl font-black uppercase tracking-wide">LLM Providers</h2>
              <p className="text-sm mt-1" style={{ color: "var(--ink-soft)" }}>
                支持多个 provider，当前只激活一个。后端会按激活项创建 `LlmClient`。
              </p>
            </div>
            <button
              onClick={() => {
                const newProvider: LlmProvider = {
                  id: "new_" + Date.now(),
                  name: "New Provider",
                  base_url: "",
                  api_key: "",
                  default_model: "gpt-4o",
                  max_tokens: 8192,
                  temperature: 1.0,
                  is_native_anthropic: false,
                  max_retries: 3,
                };
                setConfig({ ...config, providers: [...config.providers, newProvider] });
              }}
              className="flex items-center gap-2 px-4 py-2.5 border-[3px] hover:brightness-110 transition-all"
              style={{
                backgroundColor: "var(--accent)",
                color: "var(--ink)",
                borderColor: "var(--line)",
                boxShadow: "4px 4px 0 0 var(--shadow-strong)",
                borderRadius: "255px 15px 225px 15px / 15px 225px 15px 255px"
              }}
            >
              <Plus className="w-4 h-4" />
              <span className="font-black uppercase tracking-wide">Add Provider</span>
            </button>
          </div>

          <div className="space-y-6">
            {config.providers.map((provider, i) => (
              <div
                key={provider.id}
                className="p-5 border-[3px]"
                style={{
                  backgroundColor: "var(--surface-muted)",
                  borderColor: "var(--line)",
                  boxShadow: "4px 4px 0 0 var(--shadow-strong)",
                  borderRadius: "16px 26px 14px 28px / 26px 14px 28px 16px"
                }}
              >
                <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between mb-5">
                  <div className="flex items-center gap-3">
                    <input
                      type="radio"
                      name="activeProvider"
                      checked={config.active_provider_id === provider.id}
                      onChange={() => setConfig({ ...config, active_provider_id: provider.id })}
                      className="w-4 h-4"
                      style={{ accentColor: "var(--accent)" }}
                    />
                    <input
                      type="text"
                      value={provider.name}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].name = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="font-black bg-transparent border-b-[3px] outline-none px-1 py-1"
                      style={{ borderColor: "var(--line)", color: "var(--ink)" }}
                    />
                  </div>
                  <button
                    onClick={() => {
                      const newP = config.providers.filter((p) => p.id !== provider.id);
                      setConfig({ ...config, providers: newP });
                    }}
                    className="flex items-center gap-2 self-start md:self-auto px-3 py-2 border-[3px] hover:brightness-110 transition-all"
                    style={{
                      backgroundColor: "var(--danger-soft)",
                      color: "var(--ink)",
                      borderColor: "var(--line)",
                      boxShadow: "3px 3px 0 0 var(--shadow-strong)",
                      borderRadius: "18px 10px 14px 22px / 22px 14px 10px 18px"
                    }}
                  >
                    <Trash2 className="w-4 h-4" />
                    <span className="font-black uppercase text-xs tracking-wide">Remove</span>
                  </button>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div className="md:col-span-2">
                    <label className="block text-xs font-black uppercase tracking-wide mb-2">Base URL</label>
                    <input
                      type="text"
                      value={provider.base_url}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].base_url = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-3 border-[3px] outline-none theme-input"
                      style={{
                        backgroundColor: "var(--paper-bg)",
                        color: "var(--ink)",
                        borderColor: "var(--line)",
                        borderRadius: "14px 20px 12px 18px / 18px 12px 20px 14px",
                        boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                      }}
                    />
                  </div>
                  <div className="md:col-span-2">
                    <label className="block text-xs font-black uppercase tracking-wide mb-2">API Key</label>
                    <input
                      type="password"
                      value={provider.api_key}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].api_key = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-3 border-[3px] outline-none theme-input"
                      style={{
                        backgroundColor: "var(--paper-bg)",
                        color: "var(--ink)",
                        borderColor: "var(--line)",
                        borderRadius: "18px 12px 20px 14px / 14px 18px 12px 20px",
                        boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                      }}
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-black uppercase tracking-wide mb-2">Model Name</label>
                    <input
                      type="text"
                      value={provider.default_model}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].default_model = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-3 border-[3px] outline-none theme-input"
                      style={{
                        backgroundColor: "var(--paper-bg)",
                        color: "var(--ink)",
                        borderColor: "var(--line)",
                        borderRadius: "14px 18px 12px 20px / 20px 12px 18px 14px",
                        boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                      }}
                    />
                  </div>
                  <div>
                    <label className="block text-xs font-black uppercase tracking-wide mb-2">Max Retries</label>
                    <input
                      type="number"
                      value={provider.max_retries}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].max_retries = Number(e.target.value);
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-3 border-[3px] outline-none theme-input"
                      style={{
                        backgroundColor: "var(--paper-bg)",
                        color: "var(--ink)",
                        borderColor: "var(--line)",
                        borderRadius: "20px 12px 18px 14px / 14px 20px 12px 18px",
                        boxShadow: "3px 3px 0 0 var(--shadow-strong)"
                      }}
                    />
                  </div>
                  <div className="md:col-span-2 flex items-center gap-3 mt-2">
                    <input
                      type="checkbox"
                      id={`native_${provider.id}`}
                      checked={provider.is_native_anthropic}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].is_native_anthropic = e.target.checked;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-4 h-4"
                      style={{ accentColor: "var(--accent)" }}
                    />
                    <label htmlFor={`native_${provider.id}`} className="text-sm font-medium cursor-pointer">
                      Use Native Anthropic Protocol
                    </label>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
