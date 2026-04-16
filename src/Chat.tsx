import { useState, useRef, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { Send, Bot, User, Code, PlayCircle, RefreshCw, AlertTriangle } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Prism as SyntaxHighlighter } from "react-syntax-highlighter";
import { vscDarkPlus } from "react-syntax-highlighter/dist/esm/styles/prism";

interface Message {
  role: "user" | "assistant" | "system" | "tool";
  content: string;
  name?: string;
  tool_call_id?: string;
  tool_calls?: any[];
  request_id?: string;
  is_error?: boolean;
}

interface AgentEvent {
  request_id: string;
  kind: string;
  message_id?: string;
  message?: Message;
  interrupt?: boolean;
}

interface AgentRunResult {
  messages: Message[];
  interrupted: boolean;
}

function normalizeMessage(message: Message, requestId?: string): Message {
  return {
    role: message.role,
    content: message.content,
    name: message.name,
    tool_call_id: message.tool_call_id,
    tool_calls: message.tool_calls,
    request_id: requestId ?? message.request_id,
    is_error: message.is_error
  };
}

export default function Chat() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const runAgent = async (currentMessages: Message[]) => {
    const requestId = `${Date.now()}_${Math.random().toString(16).slice(2)}`;
    const unlisten = await listen<AgentEvent>("agent_event", (event) => {
      const payload = event.payload;
      if (payload.request_id !== requestId) return;

      if (payload.kind === "assistant_start" && payload.message_id) {
        setMessages((prev) => [
          ...prev,
          { role: "assistant", content: "", request_id: payload.message_id }
        ]);
        return;
      }

      if (payload.kind === "assistant_delta" && payload.message_id) {
        const delta = payload.message?.content || "";
        setMessages((prev) =>
          prev.map((message) =>
            message.request_id === payload.message_id
              ? { ...message, content: `${message.content || ""}${delta}` }
              : message
          )
        );
        return;
      }

      if (payload.kind === "assistant_done" && payload.message_id && payload.message) {
        const finalizedMessage = normalizeMessage(payload.message, payload.message_id);
        setMessages((prev) =>
          prev.map((message) =>
            message.request_id === payload.message_id
              ? finalizedMessage
              : message
          )
        );
        return;
      }

      const toolMessage = payload.message;
      if (payload.kind === "tool_result" && toolMessage) {
        setMessages((prev) => [...prev, normalizeMessage(toolMessage)]);
      }
    });

    try {
      const result = await invoke<AgentRunResult>("run_agent_stream", {
        requestId,
        messages: currentMessages,
        workspaceDir: null
      });
      setMessages(result.messages);
    } finally {
      await unlisten();
    }
  };

  const handleSend = async () => {
    if (!input.trim() || loading) return;

    const userMsg: Message = { role: "user", content: input };
    const newMessages = [...messages, userMsg];
    setMessages(newMessages);
    setInput("");
    setLoading(true);

    try {
      await runAgent(newMessages);
    } catch (e) {
      setMessages((prev) => [...prev, { role: "assistant", content: `Error: ${e}`, is_error: true }]);
    } finally {
      setLoading(false);
    }
  };

  const handleRetry = async () => {
    if (loading) return;
    
    // Remove the last error message if it exists
    let newMessages = [...messages];
    if (newMessages.length > 0 && newMessages[newMessages.length - 1].is_error) {
      newMessages.pop();
    }
    
    setMessages(newMessages);
    setLoading(true);
    
    try {
      await runAgent(newMessages);
    } catch (e) {
      setMessages((prev) => [...prev, { role: "assistant", content: `Error: ${e}`, is_error: true }]);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="flex flex-col h-full bg-transparent font-sans text-black relative">
      <div className="absolute inset-0 opacity-[0.03] pointer-events-none" style={{ backgroundImage: 'radial-gradient(#000 2px, transparent 2px)', backgroundSize: '32px 32px' }}></div>
      
      <header className="hidden"></header>

      <div className="flex-1 overflow-y-auto p-4 md:p-6 space-y-6 relative z-10">
        {messages.map((msg, idx) => {
          if (msg.role === "system") return null;

          const isUser = msg.role === "user";
          const isTool = msg.role === "tool";

          if (isTool) {
            return (
              <div key={idx} className="flex flex-col items-start gap-2 ml-16 max-w-[85%] relative group">
                <div className="absolute -left-6 top-4 w-4 h-[3px] bg-black opacity-30 group-hover:opacity-100 transition-opacity"></div>
                <div 
                  className="flex items-center gap-2 text-xs text-black font-black bg-[#ffde59] border-[3px] border-black shadow-[3px_3px_0px_0px_rgba(0,0,0,1)] px-3 py-1.5 uppercase tracking-wide" 
                  style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', filter: 'url(#wobbly-edge)' }}
                >
                  <PlayCircle className="w-4 h-4" />
                  Execution: {msg.name}
                </div>
                <div 
                  className="bg-[#f4f4f0] border-[3px] border-black text-black text-sm p-4 font-mono overflow-x-auto w-full shadow-[5px_5px_0px_0px_rgba(0,0,0,1)] relative" 
                  style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px', borderTopLeftRadius: '0px' }}
                >
                  <div className="absolute top-2 right-2 flex gap-1">
                    <div className="w-2 h-2 rounded-full border border-black bg-gray-300"></div>
                    <div className="w-2 h-2 rounded-full border border-black bg-gray-300"></div>
                  </div>
                  <pre className="mt-2 leading-relaxed">{msg.content}</pre>
                </div>
              </div>
            );
          }

          return (
            <div key={idx} className={`flex gap-5 ${isUser ? "flex-row-reverse" : "flex-row"} items-start`}>
              <div 
                className={`w-12 h-12 border-[3px] border-black flex items-center justify-center shrink-0 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] relative z-10 ${isUser ? "bg-black" : "bg-white"}`}
                style={{ 
                  borderRadius: isUser ? '15px 255px 15px 225px/225px 15px 255px 15px' : '255px 15px 225px 15px/15px 225px 15px 255px',
                  filter: 'url(#wobbly-edge)'
                }}
              >
                {isUser ? <User className="w-6 h-6 text-white" /> : <Bot className="w-7 h-7 text-black" />}
              </div>
              <div className={`flex flex-col ${isUser ? "items-end" : "items-start"} max-w-[85%] relative`}>
                {msg.content && (
                  <div 
                    className={`px-6 py-5 border-[3px] border-black shadow-[6px_6px_0px_0px_rgba(0,0,0,1)] text-base leading-relaxed relative ${
                      isUser 
                        ? "bg-white text-black" 
                        : msg.is_error 
                          ? "bg-[#ff9999] text-black"
                          : "bg-white text-black"
                    }`}
                    style={{ 
                      borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px',
                      borderTopRightRadius: isUser ? '0px' : undefined,
                      borderTopLeftRadius: !isUser ? '0px' : undefined
                    }}
                  >
                    {/* 纸张高光效果 */}
                    <div className="absolute top-1 left-1 right-1 h-2 bg-white opacity-40 rounded-full blur-[1px] pointer-events-none"></div>
                    
                    {msg.is_error && (
                      <div className="flex items-center gap-2 font-black text-black border-b-[3px] border-black pb-3 mb-4 uppercase tracking-wider text-lg">
                        <AlertTriangle className="w-6 h-6" />
                        Something went wrong
                      </div>
                    )}
                    <div className="prose prose-base max-w-none prose-p:my-3 prose-p:font-medium prose-pre:bg-[#1a1a1a] prose-pre:text-gray-100 prose-pre:border-[3px] prose-pre:border-black prose-pre:shadow-[5px_5px_0px_0px_rgba(0,0,0,1)] prose-pre:p-0 prose-pre:overflow-hidden prose-table:border-collapse prose-table:border-[3px] prose-table:border-black prose-th:border-[3px] prose-th:border-black prose-th:bg-[#ffde59] prose-th:p-3 prose-th:font-black prose-th:uppercase prose-td:border-[3px] prose-td:border-black prose-td:p-3 prose-headings:font-black prose-headings:uppercase prose-headings:tracking-tight prose-a:text-black prose-a:font-black prose-a:underline prose-a:decoration-[3px] prose-a:decoration-[#ffde59] hover:prose-a:bg-[#ffde59] prose-strong:font-black prose-li:font-medium">
                      <ReactMarkdown
                        remarkPlugins={[remarkGfm]}
                        components={{
                          code({node, inline, className, children, ...props}: any) {
                            const match = /language-(\w+)/.exec(className || "");
                            return !inline && match ? (
                              <div className="relative group">
                                <div className="absolute top-0 left-0 right-0 h-8 bg-black flex items-center px-4 justify-between">
                                  <span className="text-white text-xs font-bold uppercase tracking-wider">{match[1]}</span>
                                  <div className="flex gap-1.5">
                                    <div className="w-2.5 h-2.5 rounded-full bg-white"></div>
                                    <div className="w-2.5 h-2.5 rounded-full bg-white"></div>
                                    <div className="w-2.5 h-2.5 rounded-full bg-white"></div>
                                  </div>
                                </div>
                                <SyntaxHighlighter
                                  {...props}
                                  children={String(children).replace(/\n$/, "")}
                                  style={vscDarkPlus}
                                  language={match[1]}
                                  PreTag="div"
                                  className="!m-0 !pt-10 !pb-4 !px-4 !bg-transparent"
                                />
                              </div>
                            ) : (
                              <code {...props} className={`${className} bg-[#f4f4f0] text-black border-[2px] border-black px-1.5 py-0.5 text-sm font-bold shadow-[2px_2px_0px_0px_rgba(0,0,0,1)] mx-1 whitespace-nowrap`}>
                                {children}
                              </code>
                            );
                          }
                        }}
                      >
                        {msg.content}
                      </ReactMarkdown>
                    </div>
                    {msg.is_error && !loading && (
                      <button 
                        onClick={handleRetry}
                        className="mt-6 flex items-center gap-2 px-5 py-2.5 bg-black text-white text-sm font-black uppercase tracking-widest border-[3px] border-black hover:bg-[#ffde59] hover:text-black hover:shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] hover:-translate-y-1 transition-all"
                        style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', filter: 'url(#wobbly-edge)' }}
                      >
                        <RefreshCw className="w-5 h-5" />
                        Retry Operation
                      </button>
                    )}
                  </div>
                )}
                
                {msg.tool_calls && msg.tool_calls.map((tc, tcIdx) => (
                  <div 
                    key={tcIdx} 
                    className="mt-4 flex items-center gap-3 text-sm text-black font-black bg-white px-4 py-2.5 border-[3px] border-black shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] relative"
                    style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px', filter: 'url(#wobbly-edge)' }}
                  >
                    {/* 模拟胶带效果 */}
                    <div className="absolute -top-2 left-1/2 -translate-x-1/2 w-8 h-4 bg-white opacity-80 border border-gray-200 rotate-2 z-10"></div>
                    <Code className="w-5 h-5" />
                    <span className="uppercase tracking-wide">Executing <span className="font-mono bg-[#ffde59] px-2 py-0.5 border-2 border-black ml-1 inline-block rotate-1 shadow-[2px_2px_0px_0px_rgba(0,0,0,1)]">{tc.function.name}</span></span>
                  </div>
                ))}
              </div>
            </div>
          );
        })}
        {loading && (
          <div className="flex gap-5 items-center">
            <div 
              className="w-12 h-12 bg-white border-[3px] border-black flex items-center justify-center shrink-0 shadow-[4px_4px_0px_0px_rgba(0,0,0,1)]"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', filter: 'url(#wobbly-edge)' }}
            >
              <Bot className="w-7 h-7 text-black animate-pulse" />
            </div>
            <div 
              className="bg-white border-[3px] border-black px-6 py-5 shadow-[6px_6px_0px_0px_rgba(0,0,0,1)] flex gap-3 items-center relative"
              style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', borderTopLeftRadius: '0px' }}
            >
              <span className="text-black font-black uppercase tracking-widest text-sm mr-2">Thinking</span>
              <div className="w-3 h-3 bg-black rounded-none border border-black animate-bounce" style={{ borderRadius: '4px 6px 3px 7px' }}></div>
              <div className="w-3 h-3 bg-black rounded-none border border-black animate-bounce" style={{ borderRadius: '6px 4px 7px 3px', animationDelay: "0.2s" }}></div>
              <div className="w-3 h-3 bg-[#ffde59] rounded-none border border-black animate-bounce" style={{ borderRadius: '5px 7px 4px 6px', animationDelay: "0.4s" }}></div>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} className="h-4" />
      </div>

      <div className="p-6 md:p-8 bg-white border-t-[3px] border-black relative z-20 shadow-[0_-10px_30px_-15px_rgba(0,0,0,0.1)]">
        <div className="max-w-5xl mx-auto relative">
          {/* 输入框顶部装饰线条 */}
          <div className="absolute -top-4 left-4 right-4 h-1 bg-black opacity-20" style={{ filter: 'url(#wobbly-edge)' }}></div>
          
          <div 
            className="relative flex items-end bg-white border-[3px] border-black shadow-[6px_6px_0px_0px_rgba(0,0,0,1)] focus-within:translate-x-[2px] focus-within:translate-y-[2px] focus-within:shadow-[4px_4px_0px_0px_rgba(0,0,0,1)] transition-all z-10"
            style={{ borderRadius: '255px 15px 225px 15px/15px 225px 15px 255px', filter: 'url(#wobbly-edge)' }}
          >
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleSend();
                }
              }}
              placeholder="TYPE YOUR COMMAND HERE..."
              className="w-full max-h-64 min-h-[72px] p-5 pr-20 bg-transparent outline-none resize-none text-black text-lg font-bold placeholder-gray-400 uppercase tracking-wide"
              rows={1}
            />
            <button
              onClick={handleSend}
              disabled={!input.trim() || loading}
              className="absolute right-3 bottom-3 p-3 bg-black text-white border-[3px] border-black hover:bg-[#ffde59] hover:text-black disabled:opacity-40 disabled:hover:bg-black disabled:hover:text-white transition-colors group"
              style={{ borderRadius: '15px 255px 15px 225px/225px 15px 255px 15px' }}
            >
              <Send className="w-6 h-6 group-hover:translate-x-1 group-hover:-translate-y-1 transition-transform" />
            </button>
          </div>
          <div className="flex justify-between items-center mt-3 px-2">
            <div className="text-[10px] font-black uppercase tracking-widest text-black bg-[#ffde59] px-2 py-0.5 border-2 border-black inline-block rotate-[-1deg] shadow-[2px_2px_0px_0px_rgba(0,0,0,1)]">
              GENERIC AGENT
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
