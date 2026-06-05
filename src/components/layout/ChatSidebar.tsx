import { useEffect, useState, useRef, useCallback, useMemo } from "react";
import { useKBStore } from "@/stores/useKBStore";
import { useAppStore } from "@/stores/useAppStore";
import { useEditorStore } from "@/stores/useEditorStore";
import type { Conversation, ChatMessage } from "@/types/chat";
import type { SearchResult, WebPageContent, WebSearchConfig } from "@/types/webSearch";
import {
  Send, Loader2, Bookmark, Bot, User, Globe, Search, ExternalLink,
  ChevronDown, ChevronUp, CheckSquare, Square, Eye, FileText, Plus,
  Trash2, MessageSquare, Pencil, X, PanelRightClose,
} from "lucide-react";

import MarkdownRenderer from "@/components/common/MarkdownRenderer";

// Lazy Tauri API imports for frontend-only dev mode compatibility
async function tauriInvoke<T = any>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import("@tauri-apps/api/core");
  return invoke<T>(cmd, args);
}

async function tauriListen<T = any>(event: string, handler: (event: any) => void): Promise<() => void> {
  const { listen } = await import("@tauri-apps/api/event");
  return listen<T>(event, handler);
}

// ---- Think tag parsing ----

function parseThinkContent(text: string): {
  thinkContent: string;
  answerContent: string;
  hasCompleteThink: boolean;
} {
  // Handle both literal <think> tags and HTML-encoded &lt;think&gt; tags
  const patterns = [
    { start: "<think>", end: "</think>" },
    { start: "&lt;think&gt;", end: "&lt;/think&gt;" },
  ];

  for (const { start, end } of patterns) {
    const thinkStart = text.indexOf(start);
    if (thinkStart === -1) continue;

    const contentStart = thinkStart + start.length;
    const thinkEnd = text.indexOf(end, contentStart);

    if (thinkEnd === -1) {
      // Still streaming — everything after <think> is partial think content
      return {
        thinkContent: text.slice(contentStart),
        answerContent: text.slice(0, thinkStart).trim(),
        hasCompleteThink: false,
      };
    }

    return {
      thinkContent: text.slice(contentStart, thinkEnd).trim(),
      answerContent: (text.slice(0, thinkStart) + text.slice(thinkEnd + end.length)).trim(),
      hasCompleteThink: true,
    };
  }

  return { thinkContent: "", answerContent: text, hasCompleteThink: false };
}

// ---- ThinkBlock component ----

function ThinkBlock({ content, streaming }: { content: string; streaming: boolean }) {
  if (!content.trim()) return null;
  return (
    <details open={streaming} className="mb-2">
      <summary className="text-[10px] text-muted-foreground cursor-pointer hover:text-foreground-dim select-none">
        深度思考中...
      </summary>
      <div className="mt-1.5 px-2 py-1.5 bg-background border border-border rounded text-[10px] text-foreground-dim whitespace-pre-wrap font-sans leading-relaxed">
        {content}
        {streaming && <span className="inline-block animate-pulse text-primary">▌</span>}
      </div>
    </details>
  );
}

// ---- Message bubble helpers ----

function AssistantBubble({
  content,
  isStreaming,
  isLast,
  onSave,
  wikiSuggestion,
}: {
  content: string;
  isStreaming: boolean;
  isLast: boolean;
  onSave: (text: string) => void;
  wikiSuggestion?: { recommended: boolean; suggested_title: string; reason: string } | null;
}) {
  const { thinkContent, answerContent, hasCompleteThink } = parseThinkContent(content);
  const hasThink = !!(thinkContent || content.includes("<think>") || content.includes("&lt;think&gt;"));
  const thinkStreaming = hasThink && !hasCompleteThink && isStreaming;

  // 尝试解析 JSON 响应，提取 answer 字段进行 Markdown 渲染
  const renderedContent = useMemo(() => {
    if (isStreaming) return null; // 流式输出时不解析 JSON
    const text = answerContent || content;
    try {
      const parsed = JSON.parse(text);
      if (parsed.answer) return parsed.answer;
    } catch {}
    return null;
  }, [content, answerContent, isStreaming]);

  const displayText = renderedContent || answerContent || (!hasThink ? content : "");

  return (
    <div className="max-w-[85%] px-3 py-2 text-xs leading-relaxed rounded bg-chat-assistant-bg text-chat-assistant-text border border-border">
      {hasThink && (
        <ThinkBlock content={thinkContent} streaming={thinkStreaming} />
      )}
      {displayText ? (
        <div className="whitespace-pre-wrap font-sans">
          {renderedContent ? (
            /* 使用 Markdown 渲染 JSON 解析后的 answer 字段 */
            <MarkdownRenderer content={renderedContent} />
          ) : (
            displayText
          )}
          {isStreaming && isLast && hasCompleteThink && (
            <span className="inline-block animate-pulse text-primary">▌</span>
          )}
        </div>
      ) : !hasThink ? (
        <pre className="whitespace-pre-wrap font-sans">
          {content}
          {isStreaming && isLast && (
            <span className="inline-block animate-pulse text-primary">▌</span>
          )}
        </pre>
      ) : null}
      {isStreaming && isLast && !content && (
        <Loader2 size={14} className="animate-spin text-primary" />
      )}
      {!isStreaming && content.length > 50 && (
        <div className="flex items-center gap-2 mt-1.5">
          <button
            type="button"
            onClick={() => onSave(content)}
            className="flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
          >
            <Bookmark size={10} /> 保存为 Wiki
          </button>
          {wikiSuggestion?.recommended && (
            <span className="text-[10px] text-primary bg-primary/10 px-1.5 py-0.5 rounded">
              推荐: {wikiSuggestion.suggested_title}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

// ---- Main component ----

export default function ChatSidebar() {
  const currentKB = useKBStore((s) => s.currentKB);
  const chatSidebarVisible = useAppStore((s) => s.chatSidebarVisible);
  const toggleChatSidebar = useAppStore((s) => s.toggleChatSidebar);

  const [messages, setMessages] = useState<{ role: string; content: string }[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [messagesLoading, setMessagesLoading] = useState(false);
  const [scope, setScope] = useState("all");
  const [lastCitations, setLastCitations] = useState<any[]>([]);
  const [wikiSuggestion, setWikiSuggestion] = useState<{ recommended: boolean; suggested_title: string; reason: string } | null>(null);
  const [msg, setMsg] = useState("");
  const [error, setError] = useState("");
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const abortRef = useRef(false);
  const errorTimerRef = useRef<ReturnType<typeof setTimeout>>();

  const setTimedError = (msg: string) => {
    setError(msg);
    if (errorTimerRef.current) clearTimeout(errorTimerRef.current);
    errorTimerRef.current = setTimeout(() => setError(""), 8000);
  };

  const [conversations, setConversations] = useState<Conversation[]>([]);
  const [activeConversationId, setActiveConversationId] = useState<string | null>(null);
  const [convListCollapsed, setConvListCollapsed] = useState(false);
  const [editingTitleId, setEditingTitleId] = useState<string | null>(null);
  const [editTitleValue, setEditTitleValue] = useState("");

  // Track latest load request to prevent race conditions when switching quickly
  const loadRequestRef = useRef(0);
  // Track current conversation title for auto-title check (avoids stale closure)
  const currentConvTitleRef = useRef("");

  const [webSearchEnabled, setWebSearchEnabled] = useState(false);
  const [webSearchConfig, setWebSearchConfig] = useState<WebSearchConfig | null>(null);
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [searchLoading, setSearchLoading] = useState(false);
  const [showSearchResults, setShowSearchResults] = useState(false);
  const [selectedResultIndices, setSelectedResultIndices] = useState<Set<number>>(new Set());
  const [extractedContents, setExtractedContents] = useState<WebPageContent[]>([]);
  const [fetchingContent, setFetchingContent] = useState(false);
  const [fetchProgress, setFetchProgress] = useState("");
  const [showPreviewIndex, setShowPreviewIndex] = useState<number | null>(null);

  // Streaming state
  const [streamEnabled, setStreamEnabled] = useState(true);
  const [isStreaming, setIsStreaming] = useState(false);
  const streamingTextRef = useRef("");
  const unlistenRefs = useRef<{ chunk?: () => void; done?: () => void; error?: () => void }>({});

  // AI generation toggle — when disabled, AI must base answers strictly on wiki content
  const [allowAiGeneration, setAllowAiGeneration] = useState(true);

  useEffect(() => {
    tauriInvoke<WebSearchConfig>("get_web_search_config")
      .then(setWebSearchConfig)
      .catch(() => {});
  }, []);

  useEffect(() => {
    if (currentKB) {
      loadConversations();
      // Load KB-level AI generation setting
      tauriInvoke<any>("get_kb_stats", { kbId: currentKB.id })
        .then((s) => { if (s && typeof s.allow_ai_generation === "boolean") setAllowAiGeneration(s.allow_ai_generation); })
        .catch(() => {});
    }
  }, [currentKB]);

  const loadConversations = async () => {
    if (!currentKB) return;
    try {
      const list = await tauriInvoke<Conversation[]>("list_conversations", { kbId: currentKB.id });
      setConversations(list);
    } catch (e) {
      setTimedError(`加载对话列表失败: ${e}`);
    }
  };

  useEffect(() => {
    if (activeConversationId) {
      // Clear messages immediately when switching conversations
      setMessages([]);
      setLastCitations([]);
      setSearchResults([]);
      setShowSearchResults(false);
      setMessagesLoading(true);

      // Update title ref
      const conv = conversations.find((c) => c.id === activeConversationId);
      currentConvTitleRef.current = conv?.title || "";

      const requestId = ++loadRequestRef.current;
      loadMessages(activeConversationId).finally(() => {
        // Only stop loading if this is still the latest request
        if (loadRequestRef.current === requestId) {
          setMessagesLoading(false);
        }
      });
    } else {
      setMessages([]);
      setLastCitations([]);
      setSearchResults([]);
      setShowSearchResults(false);
      currentConvTitleRef.current = "";
    }
  }, [activeConversationId]);

  const loadMessages = async (convId: string) => {
    const requestId = loadRequestRef.current;
    try {
      const msgs = await tauriInvoke<ChatMessage[]>("get_conversation_messages", { conversationId: convId });
      // Only apply if this request hasn't been superseded
      if (loadRequestRef.current !== requestId) return;
      setMessages(msgs.map((m) => ({ role: m.role, content: m.content })));
      const lastAssistant = [...msgs].reverse().find((m) => m.role === "assistant");
      if (lastAssistant?.citations) {
        try { setLastCitations(JSON.parse(lastAssistant.citations)); } catch { setLastCitations([]); }
      } else { setLastCitations([]); }
    } catch (e) {
      if (loadRequestRef.current !== requestId) return;
      setTimedError(`加载消息失败: ${e}`);
    }
  };

  const handleNewConversation = async () => {
    if (!currentKB) return;
    try {
      const conv = await tauriInvoke<Conversation>("create_conversation", { kbId: currentKB.id, title: null });
      setConversations((prev) => [conv, ...prev]);
      setActiveConversationId(conv.id);
      currentConvTitleRef.current = conv.title || "新对话";
    } catch (e) {
      setTimedError(`创建对话失败: ${e}`);
    }
  };

  const handleDeleteConversation = async (convId: string) => {
    try {
      await tauriInvoke("delete_conversation", { conversationId: convId });
      setConversations((prev) => prev.filter((c) => c.id !== convId));
      if (activeConversationId === convId) {
        setActiveConversationId(null);
        currentConvTitleRef.current = "";
      }
    } catch (e) {
      setTimedError(`删除对话失败: ${e}`);
    }
  };

  const handleStartRename = (conv: Conversation) => {
    setEditingTitleId(conv.id);
    setEditTitleValue(conv.title);
  };

  const handleFinishRename = async (convId: string) => {
    if (editTitleValue.trim()) {
      try {
        await tauriInvoke("update_conversation_title", { conversationId: convId, title: editTitleValue.trim() });
        setConversations((prev) =>
          prev.map((c) => (c.id === convId ? { ...c, title: editTitleValue.trim() } : c))
        );
        if (convId === activeConversationId) {
          currentConvTitleRef.current = editTitleValue.trim();
        }
      } catch (e) {
        setTimedError(`重命名失败: ${e}`);
      }
    }
    setEditingTitleId(null);
  };

  const saveMessages = useCallback(
    async (convId: string, role: string, content: string, citations?: string) => {
      try {
        await tauriInvoke("save_message", {
          conversationId: convId,
          role,
          content,
          citations: citations || null,
        });
      } catch (e) {
        setTimedError(`消息保存失败: ${e instanceof Error ? e.message : String(e)}`);
      }
    },
    []
  );

  const autoTitle = useCallback(async (convId: string, userMsg: string) => {
    const title = userMsg.slice(0, 30) + (userMsg.length > 30 ? "..." : "");
    currentConvTitleRef.current = title;
    try {
      await tauriInvoke("update_conversation_title", { conversationId: convId, title });
      setConversations((prev) =>
        prev.map((c) => (c.id === convId ? { ...c, title } : c))
      );
    } catch { /* mute */ }
  }, []);

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  const cleanupListeners = useCallback(() => {
    unlistenRefs.current.chunk?.();
    unlistenRefs.current.done?.();
    unlistenRefs.current.error?.();
    unlistenRefs.current = {};
  }, []);

  useEffect(() => () => { cleanupListeners(); }, [cleanupListeners]);

  const handleSend = async () => {
    if (loading || isStreaming) return;
    if (!input.trim() || !currentKB) return;
    const userMsg = input.trim();

    let convId = activeConversationId;
    let isBrandNewConv = false;
    if (!convId) {
      try {
        const conv = await tauriInvoke<Conversation>("create_conversation", {
          kbId: currentKB.id,
          title: null,
        });
        setConversations((prev) => [conv, ...prev]);
        convId = conv.id;
        setActiveConversationId(convId);
        currentConvTitleRef.current = conv.title || "新对话";
        isBrandNewConv = true;
      } catch (e) {
        setTimedError(`创建对话失败: ${e}`);
        return;
      }
    }

    const isFirstMessage = messages.length === 0;
    abortRef.current = false;
    setMessages((prev) => [...prev, { role: "user", content: userMsg }]);
    setInput("");
    setLoading(true);
    setMsg("");
    setError("");
    if (errorTimerRef.current) { clearTimeout(errorTimerRef.current); errorTimerRef.current = undefined; }
    setSearchResults([]);
    setShowSearchResults(false);

    await saveMessages(convId, "user", userMsg);

    // Auto-title: uses ref (not stale closure) to check if conversation still has default title
    const defaultTitle = currentConvTitleRef.current;
    if (isFirstMessage && (isBrandNewConv || !defaultTitle || defaultTitle === "新对话")) {
      autoTitle(convId, userMsg);
    }

    let questionWithContext = userMsg;
    if (webSearchEnabled) {
      setSearchLoading(true);
      try {
        const results = await tauriInvoke<SearchResult[]>("web_search", {
          query: userMsg,
          engine: webSearchConfig?.engine || "duckduckgo",
          maxResults: webSearchConfig?.max_results || 10,
        });
        if (results.length > 0) {
          setSearchResults(results);
          setShowSearchResults(true);
          const searchContext = results
            .map((r, i) => `[${i + 1}] ${r.title}\nURL: ${r.url}\n摘要: ${r.snippet}`)
            .join("\n\n");
          questionWithContext = `【联网搜索结果】\n${searchContext}\n\n---\n基于以上搜索结果，请回答用户问题。如果搜索结果不足以回答，请基于你的知识补充。\n\n用户问题: ${userMsg}`;
        }
      } catch (e) {
        setTimedError(`联网搜索失败: ${e}，将使用本地知识库回答`);
      }
      setSearchLoading(false);
    }

    if (streamEnabled) {
      cleanupListeners();
      streamingTextRef.current = "";
      setMessages((prev) => [...prev, { role: "assistant", content: "" }]);
      setIsStreaming(true);

      const [unlistenChunk, unlistenDone, unlistenError] = await Promise.all([
        tauriListen<{ chunk: string; accumulated: string }>(
          "chat-stream-chunk",
          (event) => {
            if (abortRef.current) return;
            streamingTextRef.current += event.payload.chunk;
            setMessages((prev) => {
              const updated = [...prev];
              const lastIdx = updated.length - 1;
              if (updated[lastIdx]?.role === "assistant") {
                updated[lastIdx] = { ...updated[lastIdx], content: streamingTextRef.current };
              }
              return updated;
            });
          }
        ),
        tauriListen<{ full_text: string; model: string; usage: unknown }>(
          "chat-stream-done",
          async (event) => {
            cleanupListeners();
            setIsStreaming(false);
            setLoading(false);
            if (abortRef.current) return;
            const rawText = event.payload.full_text || streamingTextRef.current;
            // 尝试解析 JSON 响应，提取 answer + save_as_wiki_page
            let finalText = rawText;
            let citationsJson = "";
            try {
              const parsed = JSON.parse(rawText);
              if (parsed.answer) {
                finalText = parsed.answer; // 仅存储 answer 文本
                citationsJson = JSON.stringify(parsed.citations || []);
                if (parsed.citations?.length > 0) setLastCitations(parsed.citations);
                if (parsed.save_as_wiki_page?.recommended) {
                  setWikiSuggestion(parsed.save_as_wiki_page);
                } else {
                  setWikiSuggestion(null);
                }
              }
            } catch {}
            setMessages((prev) => {
              const updated = [...prev];
              const lastIdx = updated.length - 1;
              if (updated[lastIdx]?.role === "assistant") {
                updated[lastIdx] = { ...updated[lastIdx], content: finalText };
              }
              return updated;
            });
            await saveMessages(convId, "assistant", finalText, citationsJson || undefined);
          }
        ),
        tauriListen<{ error: string }>(
          "chat-stream-error",
          (event) => {
            cleanupListeners();
            setIsStreaming(false);
            setLoading(false);
            setMessages((prev) => {
              const updated = [...prev];
              const lastIdx = updated.length - 1;
              if (updated[lastIdx]?.role === "assistant") {
                updated[lastIdx] = {
                  ...updated[lastIdx],
                  content: `流式响应错误: ${event.payload.error}`,
                };
              }
              return updated;
            });
          }
        ),
      ]);

      unlistenRefs.current = { chunk: unlistenChunk, done: unlistenDone, error: unlistenError };

      try {
        await tauriInvoke("chat_stream", {
          systemPrompt: "",
          userContent: questionWithContext,
          kbId: currentKB.id,
          scope: scope,
          allowAiGeneration: allowAiGeneration,
        });
      } catch (e) {
        if (abortRef.current) return;
        cleanupListeners();
        setIsStreaming(false);
        setLoading(false);
        setTimedError(`启动流式对话失败: ${String(e)}`);
      }
    } else {
      try {
        const response = await tauriInvoke<string>("run_query", {
          kbId: currentKB.id,
          question: questionWithContext,
          scope,
        });
        if (abortRef.current) return;
        let displayContent = response;
        let citationsJson = "";
        try {
          const parsed = JSON.parse(response);
          if (parsed.answer) {
            displayContent = parsed.answer;
            if (parsed.citations?.length > 0) {
              setLastCitations(parsed.citations);
              citationsJson = JSON.stringify(parsed.citations);
            } else {
              setLastCitations([]);
            }
            if (parsed.save_as_wiki_page?.recommended) {
              setWikiSuggestion(parsed.save_as_wiki_page);
            } else {
              setWikiSuggestion(null);
            }
          } else {
            setLastCitations([]);
            setWikiSuggestion(null);
          }
        } catch {
          setLastCitations([]);
        }
        setMessages((prev) => [...prev, { role: "assistant", content: displayContent }]);
        await saveMessages(convId, "assistant", displayContent, citationsJson || undefined);
      } catch (e) {
        setTimedError(`问答失败: ${String(e)}`);
      }
      setLoading(false);
    }
  };

  const handleStop = () => {
    abortRef.current = true;
    setIsStreaming(false);
    setLoading(false);
    cleanupListeners();
    setMessages((prev) => {
      const updated = [...prev];
      const lastIdx = updated.length - 1;
      if (updated[lastIdx]?.role === "assistant") {
        const content = updated[lastIdx].content;
        if (content && !content.endsWith("[已中断]")) {
          updated[lastIdx] = { ...updated[lastIdx], content: content + "\n\n[已中断]" };
        } else if (!content) {
          updated[lastIdx] = { ...updated[lastIdx], content: "[已中断]" };
        }
      }
      return updated;
    });
    setMsg("已停止生成");
  };

  const handleSaveAsWiki = async (content: string) => {
    if (!currentKB) return;
    try {
      const title =
        content.split("\n")[0].replace(/^#+\s*/, "").trim().slice(0, 50) || "问答记录";
      await tauriInvoke("save_answer_as_wiki", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        title,
        content,
      });
      setMsg("已保存为 Wiki 页面");
    } catch (e) {
      setTimedError(`保存失败: ${e}`);
    }
  };

  const handleFetchSelectedContent = async () => {
    if (selectedResultIndices.size === 0) return;
    setFetchingContent(true);
    setExtractedContents([]);
    abortRef.current = false;
    const selectedUrls = Array.from(selectedResultIndices).map((i) => searchResults[i].url);
    const contents: WebPageContent[] = [];
    let done = 0;
    for (const url of selectedUrls) {
      if (abortRef.current) break;
      done++;
      const sr = searchResults.find((r) => r.url === url);
      setFetchProgress(
        `正在分析 ${done}/${selectedUrls.length}: ${sr?.title?.slice(0, 30) || url.slice(0, 30)}...`
      );
      try {
        const result = await Promise.race([
          tauriInvoke<WebPageContent>("fetch_web_page_content", { url }),
          new Promise<never>((_, reject) => setTimeout(() => reject(new Error("请求超时")), 30000)),
        ]);
        contents.push(result);
      } catch (e) {
        contents.push({
          title: sr?.title || url,
          url,
          content: `提取失败: ${e}`,
          content_length: 0,
        });
      }
    }
    setFetchProgress("");
    setExtractedContents(contents);
    setFetchingContent(false);
  };

  const handleSaveExtractedContent = async (item: WebPageContent) => {
    if (!currentKB) return;
    try {
      const now = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
      const markdown = `# 网页内容: ${item.title}\n\n> 来源: ${item.url} | 提取时间: ${now} | 内容长度: ${item.content_length} 字符\n\n---\n\n${item.content}`;
      await tauriInvoke("save_web_result_as_source", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        title: `网页内容: ${item.title.slice(0, 40)}`,
        content: markdown,
        format: "md",
      });
      setMsg(`${item.title} 已保存为知识库源文件`);
    } catch (e) {
      setTimedError(`保存失败: ${e}`);
    }
  };

  const handleSaveSearchAsSource = async () => {
    if (!currentKB || searchResults.length === 0) return;
    try {
      const query = messages.filter((m) => m.role === "user").pop()?.content || "网页搜索";
      const now = new Date().toISOString().replace(/[:.]/g, "-").slice(0, 19);
      const content = `# 网页搜索结果: ${query}\n\n> 搜索时间: ${now} | 结果数量: ${searchResults.length}\n\n---\n${searchResults.map((r, i) => `## ${i + 1}. ${r.title}\n- **URL**: ${r.url}\n- **摘要**: ${r.snippet}\n`).join("\n")}`;
      await tauriInvoke("save_web_result_as_source", {
        kbId: currentKB.id,
        kbPath: currentKB.path,
        title: `网页搜索: ${query.slice(0, 40)}`,
        content,
        format: "md",
      });
      setMsg("搜索结果已保存为知识库源文件");
    } catch (e) {
      setTimedError(`保存搜索结果失败: ${e}`);
    }
  };

  if (!chatSidebarVisible) return null;

  return (
    <div className="w-[350px] h-full bg-card border-l border-border flex flex-col shrink-0">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border shrink-0">
        <div className="flex items-center gap-2">
          <Bot size={16} className="text-primary" />
          <span className="text-sm font-medium text-foreground">AI 助手</span>
        </div>
        <button
          type="button"
          onClick={toggleChatSidebar}
          className="p-1 hover:bg-card-hover text-muted-foreground hover:text-foreground transition-colors rounded"
          title="关闭面板"
        >
          <PanelRightClose size={15} />
        </button>
      </div>

      {!currentKB ? (
        <div className="flex-1 flex flex-col items-center justify-center text-sm text-muted-foreground px-4 text-center gap-3">
          <span>请先创建或选择一个知识库后开始问答</span>
          <button
            type="button"
            onClick={() => {
              const { openFile } = useEditorStore.getState();
              openFile({ path: "settings", title: "设置", type: "settings" });
            }}
            className="px-4 py-1.5 text-xs bg-primary text-primary-foreground hover:bg-primary-hover rounded transition-colors"
          >
            前往设置创建知识库
          </button>
        </div>
      ) : (
        <>
          {/* Conversation list */}
          <div
            className={`border-b border-border shrink-0 transition-all ${convListCollapsed ? "h-auto" : "max-h-[200px] flex flex-col"}`}
          >
            <div className="flex items-center justify-between px-4 py-2">
              <button
                type="button"
                onClick={() => setConvListCollapsed(!convListCollapsed)}
                className="flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground"
              >
                <MessageSquare size={12} />
                <span>对话历史</span>
                {convListCollapsed ? (
                  <ChevronDown size={12} />
                ) : (
                  <ChevronUp size={12} />
                )}
              </button>
              <button
                type="button"
                onClick={handleNewConversation}
                className="p-1 hover:bg-card-hover text-muted-foreground hover:text-foreground rounded"
                title="新建对话"
              >
                <Plus size={13} />
              </button>
            </div>
            {!convListCollapsed && (
              <div className="overflow-y-auto flex-1 px-3 pb-2 space-y-0.5">
                {conversations.length === 0 ? (
                  <p className="text-xs text-muted-foreground text-center py-4">暂无对话记录</p>
                ) : (
                  conversations.map((conv) => (
                    <div
                      key={conv.id}
                      className={`group flex items-center gap-1.5 px-2 py-1.5 cursor-pointer rounded transition-colors ${
                        activeConversationId === conv.id
                          ? "bg-card-active text-foreground"
                          : "hover:bg-card-hover text-foreground-dim"
                      }`}
                      role="button"
                      tabIndex={0}
                      aria-label={`切换到对话: ${conv.title}`}
                      onClick={() => setActiveConversationId(conv.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          setActiveConversationId(conv.id);
                        }
                      }}
                    >
                      {editingTitleId === conv.id ? (
                        <input
                          value={editTitleValue}
                          onChange={(e) => setEditTitleValue(e.target.value)}
                          onBlur={() => handleFinishRename(conv.id)}
                          onKeyDown={(e) => {
                            if (e.key === "Enter") handleFinishRename(conv.id);
                            if (e.key === "Escape") setEditingTitleId(null);
                          }}
                          onClick={(e) => e.stopPropagation()}
                          className="flex-1 text-xs bg-background border border-border px-1.5 py-0.5 outline-none text-foreground rounded"
                          autoFocus
                        />
                      ) : (
                        <span className="flex-1 text-xs truncate">{conv.title}</span>
                      )}
                      {activeConversationId === conv.id && editingTitleId !== conv.id && (
                        <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                          <button
                            tabIndex={-1}
                            onClick={(e) => {
                              e.stopPropagation();
                              handleStartRename(conv);
                            }}
                            className="p-0.5 hover:bg-muted text-muted-foreground rounded"
                            title="重命名"
                          >
                            <Pencil size={10} />
                          </button>
                          <button
                            tabIndex={-1}
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDeleteConversation(conv.id);
                            }}
                            className="p-0.5 hover:bg-destructive/30 text-destructive rounded"
                            title="删除"
                          >
                            <Trash2 size={10} />
                          </button>
                        </div>
                      )}
                    </div>
                  ))
                )}
              </div>
            )}
          </div>

          {/* Messages area */}
          <div
            className="flex-1 overflow-y-auto px-4 py-3 space-y-3"
            role="log"
            aria-live="polite"
            aria-label="消息列表"
          >
            {error && (
              <div className="text-xs text-destructive bg-destructive-subtle px-2 py-1 rounded">{error}</div>
            )}
            {msg && (
              <div className="text-xs text-foreground-dim bg-card-hover px-2 py-1 rounded">{msg}</div>
            )}

            {/* Search results */}
            {(searchResults.length > 0 || searchLoading) && (
              <div className="border border-border rounded">
                <button
                  type="button"
                  onClick={() => setShowSearchResults(!showSearchResults)}
                  className="w-full flex items-center justify-between px-3 py-2 hover:bg-card-hover transition-colors rounded"
                >
                  <div className="flex items-center gap-2 text-xs text-foreground-dim">
                    <Globe size={13} />
                    {searchLoading ? (
                      <span className="flex items-center gap-1">
                        <Loader2 size={11} className="animate-spin" />
                        搜索中...
                      </span>
                    ) : (
                      <span>搜索结果 / {searchResults.length} 条</span>
                    )}
                  </div>
                  {!searchLoading &&
                    (showSearchResults ? (
                      <ChevronUp size={13} className="text-muted-foreground" />
                    ) : (
                      <ChevronDown size={13} className="text-muted-foreground" />
                    ))}
                </button>
                {showSearchResults && !searchLoading && (
                  <div className="border-t border-border px-3 py-2 space-y-1.5 max-h-[300px] overflow-y-auto">
                    <div className="flex items-center justify-between text-[10px]">
                      <div className="flex items-center gap-2">
                        <button
                          onClick={() =>
                            setSelectedResultIndices(new Set(searchResults.map((_, j) => j)))
                          }
                          className="text-foreground-dim hover:underline"
                        >
                          全选
                        </button>
                        <button
                          onClick={() => setSelectedResultIndices(new Set())}
                          className="text-muted-foreground hover:underline"
                        >
                          取消
                        </button>
                      </div>
                      {selectedResultIndices.size > 0 && (
                        <span className="text-muted-foreground">
                          已选 {selectedResultIndices.size}/{searchResults.length}
                        </span>
                      )}
                    </div>
                    {searchResults.map((r, i) => (
                      <div key={i} className="flex items-start gap-1.5 text-xs">
                        <button
                          onClick={() => {
                            const next = new Set(selectedResultIndices);
                            next.has(i) ? next.delete(i) : next.add(i);
                            setSelectedResultIndices(next);
                          }}
                          className="mt-0.5 shrink-0 text-muted-foreground hover:text-foreground"
                        >
                          {selectedResultIndices.has(i) ? (
                            <CheckSquare size={12} />
                          ) : (
                            <Square size={12} />
                          )}
                        </button>
                        <div className="min-w-0">
                          <a
                            href={r.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="text-foreground hover:underline flex items-center gap-1"
                          >
                            {i + 1}. {r.title}{" "}
                            <ExternalLink size={9} className="text-muted-foreground" />
                          </a>
                          <p className="text-muted-foreground mt-0.5 line-clamp-2">{r.snippet}</p>
                        </div>
                      </div>
                    ))}
                    <div className="flex items-center gap-2 pt-1">
                      <button
                        onClick={handleFetchSelectedContent}
                        disabled={selectedResultIndices.size === 0 || fetchingContent}
                        className="flex items-center gap-1 px-2 py-1 text-[10px] bg-primary text-primary-foreground hover:bg-primary-hover disabled:opacity-40 rounded transition-colors"
                      >
                        {fetchingContent ? (
                          <>
                            <Loader2 size={10} className="animate-spin" />
                            {fetchProgress || "提取中..."}
                          </>
                        ) : (
                          <>
                            <Search size={10} />
                            分析选中 ({selectedResultIndices.size})
                          </>
                        )}
                      </button>
                      <button
                        onClick={handleSaveSearchAsSource}
                        className="flex items-center gap-1 px-2 py-1 text-[10px] text-foreground-dim border border-border hover:bg-card-hover rounded transition-colors"
                      >
                        <Bookmark size={10} />
                        保存结果
                      </button>
                    </div>
                    {extractedContents.length > 0 && (
                      <div className="space-y-1.5">
                        <div className="text-[10px] text-muted-foreground pt-1 border-t border-border">
                          已提取 {extractedContents.filter((c) => c.content_length > 0).length}/
                          {extractedContents.length} 个网页
                        </div>
                        {extractedContents.map((item, i) => (
                          <div key={i} className="border border-border rounded">
                            <div className="flex items-center justify-between px-2 py-1.5 bg-card-hover">
                              <span className="text-[10px] text-foreground truncate flex-1">
                                {item.title}
                              </span>
                              <div className="flex items-center gap-1 shrink-0 ml-1">
                                {item.content_length > 0 && (
                                  <>
                                    <button
                                      onClick={() =>
                                        setShowPreviewIndex(showPreviewIndex === i ? null : i)
                                      }
                                      className="p-0.5 hover:bg-muted text-muted-foreground rounded"
                                      title="预览"
                                    >
                                      <Eye size={10} />
                                    </button>
                                    <button
                                      onClick={() => handleSaveExtractedContent(item)}
                                      className="p-0.5 hover:bg-muted text-muted-foreground rounded"
                                      title="保存"
                                    >
                                      <Bookmark size={10} />
                                    </button>
                                  </>
                                )}
                              </div>
                            </div>
                            {showPreviewIndex === i && item.content_length > 0 && (
                              <div className="border-t border-border px-2 py-1.5 max-h-32 overflow-y-auto">
                                <pre className="text-[10px] text-foreground-dim whitespace-pre-wrap font-sans leading-relaxed">
                                  {item.content.slice(0, 1000)}
                                  {item.content.length > 1000 && (
                                    <span className="text-muted-foreground">
                                      {"\n"}... [显示前 1000 字符]
                                    </span>
                                  )}
                                </pre>
                              </div>
                            )}
                          </div>
                        ))}
                      </div>
                    )}
                  </div>
                )}
              </div>
            )}

            {/* Messages loading indicator */}
            {messagesLoading && (
              <div className="flex items-center justify-center py-8">
                <Loader2 size={16} className="animate-spin text-muted-foreground" />
                <span className="ml-2 text-xs text-muted-foreground">加载消息中...</span>
              </div>
            )}

            {/* Messages */}
            {!messagesLoading && messages.length === 0 ? (
              <div className="flex items-center justify-center h-full">
                <div className="text-center">
                  <Bot size={24} className="text-muted-foreground mx-auto mb-3" />
                  <p className="text-xs text-muted-foreground">基于本地 Wiki 知识库问答</p>
                  <p className="text-[10px] text-muted-foreground mt-1">支持检索范围 / 联网搜索 / 流式输出</p>
                </div>
              </div>
            ) : (
              !messagesLoading &&
              messages.map((m, i) => (
                <div
                  key={i}
                  className={`flex gap-2 ${m.role === "user" ? "flex-row-reverse" : ""}`}
                >
                  <div className="w-6 h-6 flex items-center justify-center shrink-0 mt-0.5">
                    {m.role === "assistant" ? (
                      <Bot size={13} className="text-primary" />
                    ) : (
                      <User size={12} className="text-muted-foreground" />
                    )}
                  </div>
                  {m.role === "assistant" ? (
                    <AssistantBubble
                      content={m.content}
                      isStreaming={isStreaming}
                      isLast={i === messages.length - 1}
                      onSave={handleSaveAsWiki}
                      wikiSuggestion={i === messages.length - 1 ? wikiSuggestion : null}
                    />
                  ) : (
                    <div className="max-w-[85%] px-3 py-2 text-xs leading-relaxed rounded bg-chat-user-bg text-chat-user-text">
                      <pre className="whitespace-pre-wrap font-sans">{m.content}</pre>
                    </div>
                  )}
                </div>
              ))
            )}
            {loading && !isStreaming && (
              <div className="flex gap-2">
                <div className="w-6 h-6 flex items-center justify-center shrink-0">
                  <Bot size={13} className="text-primary" />
                </div>
                <div className="bg-card-hover border border-border px-3 py-2 rounded">
                  <Loader2 size={14} className="animate-spin text-primary" />
                </div>
              </div>
            )}
            <div ref={messagesEndRef} />
          </div>

          {/* Input area */}
          <div className="border-t border-border px-4 py-3 shrink-0 space-y-2">
            <div className="flex items-center gap-1.5">
              <select
                value={scope}
                onChange={(e) => setScope(e.target.value)}
                className="px-2 py-1.5 text-[10px] border border-input-border bg-input-bg text-foreground-dim outline-none rounded"
                title="检索范围"
              >
                <option value="all">全部</option>
                <option value="tag:concept">概念</option>
                <option value="tag:entity">实体</option>
                <option value="tag:topic">主题</option>
                <option value="tag:question">问题</option>
                <option value="tag:method">方法</option>
                <option value="tag:dataset">数据集</option>
              </select>
              <button
                type="button"
                onClick={() => setWebSearchEnabled(!webSearchEnabled)}
                className={`p-1.5 border rounded transition-all shrink-0 ${
                  webSearchEnabled
                    ? "bg-primary/20 border-primary/40 text-primary"
                    : "border-border text-muted-foreground hover:text-foreground"
                }`}
                title={webSearchEnabled ? "联网搜索已开启" : "开启联网搜索"}
              >
                <Globe size={14} />
              </button>
              <button
                type="button"
                onClick={() => setStreamEnabled(!streamEnabled)}
                disabled={loading}
                className={`p-1.5 border rounded transition-all shrink-0 text-[10px] ${
                  streamEnabled
                    ? "bg-success-subtle border-success/30 text-success"
                    : "border-border text-muted-foreground hover:text-foreground"
                } disabled:opacity-50`}
                title={streamEnabled ? "流式输出已开启" : "开启流式输出"}
              >
                流式
              </button>
              <button
                type="button"
                onClick={() => setAllowAiGeneration(!allowAiGeneration)}
                className="p-1.5 border rounded transition-all shrink-0 text-[10px]"
                style={{
                  backgroundColor: allowAiGeneration ? 'var(--primary-subtle)' : 'var(--warning-subtle)',
                  borderColor: allowAiGeneration ? 'var(--primary)' : 'var(--warning)',
                  color: allowAiGeneration ? 'var(--primary)' : 'var(--warning)',
                }}
                title={allowAiGeneration ? "AI可自主生成内容 — 点击切换为仅基于Wiki回答" : "仅基于Wiki回答 — 点击切换为允许AI自主生成"}
              >
                {allowAiGeneration ? "AI+" : "Wiki"}
              </button>
            </div>
            <div className="flex items-center gap-1.5">
              <input
                value={input}
                onChange={(e) => setInput(e.target.value)}
                onKeyDown={(e) =>
                  e.key === "Enter" && !e.shiftKey && (e.preventDefault(), handleSend())
                }
                placeholder="输入问题..."
                className="flex-1 px-3 py-2 text-xs border border-input-border bg-input-bg text-foreground outline-none focus:border-primary/50 rounded placeholder:text-input-placeholder"
                disabled={loading || isStreaming}
              />
              {loading ? (
                <button
                  onClick={handleStop}
                  className="p-2 bg-destructive text-destructive-foreground hover:bg-red-600 rounded transition-colors shrink-0"
                  title="停止生成"
                >
                  <X size={15} />
                </button>
              ) : (
                <button
                  onClick={handleSend}
                  disabled={!input.trim()}
                  className="p-2 bg-primary text-primary-foreground hover:bg-primary-hover disabled:opacity-40 rounded transition-colors shrink-0"
                  title="发送"
                >
                  <Send size={15} />
                </button>
              )}
            </div>
          </div>
        </>
      )}
    </div>
  );
}
