1) System Architecture (Updated)

Runtime (Rust)

    PTY Host

    Renderer (GPU-accelerated)

    Editor (history, completion)

    Block Store (SQLite)

    Router (Shell vs AI)

    AI Bridge (IPC to agents)

    Workflow Engine (YAML)

    Privacy Layer (redaction engine)

    Search

    ContextBuilder (new) → Gathers prompt, blocks, env, file refs, redactions, metrics.

    ProvenanceLog (new) → Logs context hashes + redaction summaries.

Agent Sidecars (Python/CLI)

    Local LLM runtimes (Ollama/llama.cpp)

    Agent adapters (Open Interpreter, AutoGen)

    Stable JSON Tool API

UI Components

    Ghost Block

    Active Error Banner

    Context Used Drawer (new) — collapsible panel showing exactly what will be sent to AI.