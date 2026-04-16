import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Save, Plus, Trash2 } from "lucide-react";

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

export default function Settings() {
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

  if (loading || !config) return <div className="p-8 text-gray-500">Loading settings...</div>;

  return (
    <div className="p-8 h-full overflow-y-auto bg-gray-50 text-gray-800 font-sans">
      <div className="max-w-4xl mx-auto space-y-8 pb-12">
        <div className="flex justify-between items-center border-b pb-4">
          <h1 className="text-3xl font-light text-gray-900">Agent Settings</h1>
          <button
            onClick={saveConfig}
            className="flex items-center gap-2 bg-gray-900 text-white px-5 py-2 rounded-full hover:bg-gray-800 transition-colors shadow-sm"
          >
            <Save className="w-4 h-4" /> Save
          </button>
        </div>

        <section className="bg-white p-6 rounded-2xl shadow-sm border border-gray-100">
          <h2 className="text-xl font-medium mb-4 text-gray-800">General</h2>
          <div className="grid gap-4">
            <div>
              <label className="block text-sm font-medium text-gray-600 mb-1">Workspace Directory</label>
              <input
                type="text"
                value={config.workspace_dir}
                onChange={(e) => setConfig({ ...config, workspace_dir: e.target.value })}
                className="w-full p-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 outline-none"
              />
            </div>
            <div>
              <label className="block text-sm font-medium text-gray-600 mb-1">Memory Directory</label>
              <input
                type="text"
                value={config.memory_dir}
                onChange={(e) => setConfig({ ...config, memory_dir: e.target.value })}
                className="w-full p-2 border border-gray-200 rounded-lg focus:ring-2 focus:ring-blue-500 outline-none"
              />
            </div>
          </div>
        </section>

        <section className="bg-white p-6 rounded-2xl shadow-sm border border-gray-100">
          <div className="flex justify-between items-center mb-6">
            <h2 className="text-xl font-medium text-gray-800">LLM Providers</h2>
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
              className="flex items-center gap-1 text-sm text-blue-600 hover:text-blue-700"
            >
              <Plus className="w-4 h-4" /> Add Provider
            </button>
          </div>

          <div className="space-y-6">
            {config.providers.map((provider, i) => (
              <div key={provider.id} className="p-5 border border-gray-100 rounded-xl bg-gray-50/50">
                <div className="flex justify-between items-center mb-4">
                  <div className="flex items-center gap-3">
                    <input
                      type="radio"
                      name="activeProvider"
                      checked={config.active_provider_id === provider.id}
                      onChange={() => setConfig({ ...config, active_provider_id: provider.id })}
                      className="w-4 h-4 text-blue-600"
                    />
                    <input
                      type="text"
                      value={provider.name}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].name = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="font-medium bg-transparent border-b border-transparent hover:border-gray-300 focus:border-blue-500 outline-none px-1"
                    />
                  </div>
                  <button
                    onClick={() => {
                      const newP = config.providers.filter((p) => p.id !== provider.id);
                      setConfig({ ...config, providers: newP });
                    }}
                    className="text-gray-400 hover:text-red-500 transition-colors"
                  >
                    <Trash2 className="w-4 h-4" />
                  </button>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="col-span-2">
                    <label className="block text-xs text-gray-500 mb-1">Base URL</label>
                    <input
                      type="text"
                      value={provider.base_url}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].base_url = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-2 border border-gray-200 rounded-lg text-sm"
                    />
                  </div>
                  <div className="col-span-2">
                    <label className="block text-xs text-gray-500 mb-1">API Key</label>
                    <input
                      type="password"
                      value={provider.api_key}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].api_key = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-2 border border-gray-200 rounded-lg text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-xs text-gray-500 mb-1">Model Name</label>
                    <input
                      type="text"
                      value={provider.default_model}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].default_model = e.target.value;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-2 border border-gray-200 rounded-lg text-sm"
                    />
                  </div>
                  <div>
                    <label className="block text-xs text-gray-500 mb-1">Max Retries</label>
                    <input
                      type="number"
                      value={provider.max_retries}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].max_retries = Number(e.target.value);
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-full p-2 border border-gray-200 rounded-lg text-sm"
                    />
                  </div>
                  <div className="col-span-2 flex items-center gap-2 mt-2">
                    <input
                      type="checkbox"
                      id={`native_${provider.id}`}
                      checked={provider.is_native_anthropic}
                      onChange={(e) => {
                        const newP = [...config.providers];
                        newP[i].is_native_anthropic = e.target.checked;
                        setConfig({ ...config, providers: newP });
                      }}
                      className="w-4 h-4 rounded border-gray-300 text-blue-600"
                    />
                    <label htmlFor={`native_${provider.id}`} className="text-sm text-gray-700 cursor-pointer">
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
