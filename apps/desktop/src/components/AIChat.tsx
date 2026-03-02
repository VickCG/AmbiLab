import { useState, useRef, useEffect } from "react";
import { VscRobot, VscSend } from "react-icons/vsc";

interface Message {
  role: "user" | "assistant";
  content: string;
}

interface Props {
  currentFile: {
    path: string;
    name: string;
    content: string;
  } | null;
}

function AIChat({ currentFile }: Props) {
  const [messages, setMessages] = useState<Message[]>([
    {
      role: "assistant",
      content:
        "Hello! I'm your AI assistant. I can help you with code analysis, SQL queries, and more. What would you like to know?",
    },
  ]);
  const [input, setInput] = useState("");
  const [isLoading, setIsLoading] = useState(false);
  const messagesEndRef = useRef<HTMLDivElement>(null);

  const scrollToBottom = () => {
    messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
  };

  useEffect(() => {
    scrollToBottom();
  }, [messages]);

  const handleSend = async () => {
    if (!input.trim() || isLoading) return;

    const userMessage = input.trim();
    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: userMessage }]);
    setIsLoading(true);

    setTimeout(() => {
      let response =
        "I understand your question. Let me analyze that for you.\n\n";

      if (currentFile) {
        response += `I can see you're working on **${currentFile.name}**. `;
      }

      if (
        userMessage.toLowerCase().includes("sql") ||
        userMessage.toLowerCase().includes("query")
      ) {
        response +=
          "For SQL analysis, I can help you:\n\n" +
          "- Detect potential join explosions\n" +
          "- Estimate query cardinality\n" +
          "- Identify missing indexes\n" +
          "- Suggest query optimizations";
      } else if (
        userMessage.toLowerCase().includes("explain") ||
        userMessage.toLowerCase().includes("what")
      ) {
        response +=
          "I'd be happy to explain the code. Could you point me to a specific function or section you'd like me to analyze?";
      } else {
        response +=
          "I'm here to help with your code and SQL queries. Feel free to ask me about:\n\n" +
          "- Code explanations\n" +
          "- SQL query analysis\n" +
          "- Performance optimization\n" +
          "- Best practices";
      }

      setMessages((prev) => [...prev, { role: "assistant", content: response }]);
      setIsLoading(false);
    }, 1000);
  };

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  };

  const formatMessage = (content: string) => {
    const parts = content.split(/(`[^`]+`|\*\*[^*]+\*\*)/g);

    return parts.map((part, index) => {
      if (part.startsWith("`") && part.endsWith("`")) {
        return (
          <code
            key={index}
            style={{
              background: "var(--bg-primary)",
              padding: "2px 6px",
              borderRadius: "4px",
              fontFamily: "monospace",
            }}
          >
            {part.slice(1, -1)}
          </code>
        );
      }
      if (part.startsWith("**") && part.endsWith("**")) {
        return <strong key={index}>{part.slice(2, -2)}</strong>;
      }
      return part;
    });
  };

  return (
    <div className="ai-chat">
      <div className="ai-chat-header">
        <VscRobot className="ai-chat-header-icon" size={20} />
        <span>AI Assistant</span>
      </div>

      <div className="ai-chat-messages">
        {messages.map((msg, index) => (
          <div key={index} className={`chat-message ${msg.role}`}>
            <div className="chat-message-content">
              {msg.content.split("\n").map((line, i) => (
                <p key={i} style={{ marginBottom: line ? "8px" : "0" }}>
                  {formatMessage(line)}
                </p>
              ))}
            </div>
          </div>
        ))}
        {isLoading && (
          <div className="chat-message assistant">
            <div className="chat-message-content">
              <span style={{ opacity: 0.7 }}>Thinking...</span>
            </div>
          </div>
        )}
        <div ref={messagesEndRef} />
      </div>

      <div className="ai-chat-input">
        <div className="chat-input-wrapper">
          <textarea
            className="chat-input"
            placeholder="Ask me anything..."
            value={input}
            onChange={(e) => setInput(e.target.value)}
            onKeyDown={handleKeyDown}
            rows={1}
          />
          <button
            className="chat-send-btn"
            onClick={handleSend}
            disabled={!input.trim() || isLoading}
          >
            <VscSend />
          </button>
        </div>
      </div>
    </div>
  );
}

export default AIChat;
