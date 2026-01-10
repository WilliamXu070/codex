# Codex Project Summary

## Overview
Codex is a personalized AI assistant that never forgets. Built on a Rust-based agent core (`codex-rs`), Codex maintains a continuous, evolving memory through a novel context file system. Unlike traditional AI assistants with isolated conversations, Codex builds a persistent understanding of you, your work, and your interests—all accessible through natural language in a single, continuous conversation.

The Electron desktop app (`apps/codex-electron`) provides a Finder-style file browser and an intelligent prompt panel, enabling seamless interaction with your AI companion that remembers everything and continuously learns from your enabled directories and interactions.

## Key Components
- `codex-rs/`: Rust workspace containing the core agent, app-server, and supporting crates.
- `codex-cli/`: Node launcher for packaged Codex binaries (vendor-based; not used for dev builds).
- `apps/codex-electron/`: Electron + React/Vite desktop UI that runs a local Codex app-server in the background.
- `docs/`: Documentation for CLI, MCP interface, and project behaviors.

## Desktop App (Electron)
- Frontend: React + Vite (renderer) with Finder-like browsing and a bottom prompt panel.
- Backend: Electron main process spawns the Rust `codex-app-server` binary and speaks JSON-RPC over stdio.
- Debug: In-app debug stream shows build steps, requests, responses, and errors.
- Hidden files: dotfiles and Docker-related folders are suppressed in the file list.

## How To Run (Dev)
From repo root:

```bash
corepack pnpm --filter codex-electron dev
```

Notes:
- The first run builds `codex-app-server` from source and can take a while.
- Ensure Rust/Cargo is installed and visible to Electron (PATH includes `~/.cargo/bin`).

## Current Status
- **Core Infrastructure**: App-server binary built from `codex-rs/` with JSON-RPC API
- **Desktop App**: Electron app with Finder-style file browser and chat interface
- **Foundation**: Basic agent capabilities, file system access, and conversation handling
- **Next**: Implementing context file system and tool generation framework

---

## Vision & Core Features

### 1) Personalized, Ever-Remembering AI with Context File System

#### Vision

Codex is a **single, continuous conversation** with an AI that never forgets. Unlike traditional AI assistants that treat each conversation as isolated, Codex maintains a persistent, evolving memory through a novel context file system that the AI itself generates and refines.

**Core Principle**: One conversation, infinite memory. The AI builds and maintains its own contextual understanding of you, your work, your interests, and your history—all accessible through natural language queries.

#### How It Works: Search Engine-Style Retrieval

The system operates like a search engine with semantic understanding:

1. **Bulk Memory Storage**: All conversations, file contents, and user interactions are stored in a structured knowledge base
2. **Key Concept Extraction**: The AI identifies and maintains high-level concepts (topics, themes, relationships) as searchable indices
3. **Intelligent Retrieval**: When you ask a question, the AI:
   - Identifies relevant concepts from your query
   - Retrieves the full context for those concepts
   - Synthesizes the information to answer your question

#### Example Use Cases

**Simple Query**: "What's my friend Sarah's birthday?"
- AI identifies concept: `friends` → `sarah`
- Retrieves full `friends` context file
- Parses and returns: "Sarah's birthday is March 15th"

**Complex Query**: "Help me develop a resume"
- AI identifies multiple concepts: `projects`, `research`, `awards`, `work-experience`
- Retrieves and synthesizes context from:
  - All relevant project files
  - Research notes and publications
  - Awards and achievements
  - Work history and skills
- Generates a comprehensive, contextually-aware resume

#### Context File Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    Context File System                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│  │   hobbies    │  │   projects   │  │   research   │          │
│  │              │  │              │  │              │          │
│  │ - coding     │  │ - codex-app  │  │ - ai-memory  │          │
│  │ - emails     │  │ - web-app    │  │ - context-   │          │
│  │ - research   │  │ - cli-tool   │  │   systems    │          │
│  │ - projects   │  │              │  │              │          │
│  │ - friends    │  │              │  │              │          │
│  └──────────────┘  └──────────────┘  └──────────────┘          │
│                                                                   │
│  Each concept file contains:                                      │
│  - Structured metadata (dates, relationships, tags)              │
│  - Full content references (file paths, conversation IDs)         │
│  - Semantic embeddings for retrieval                              │
│  - Cross-references to related concepts                           │
└─────────────────────────────────────────────────────────────────┘
```

#### Directory-Based Context Building

Users control what the AI can access:

1. **Directory Selection**: User enables specific directories on their machine
   - Example: `~/Documents`, `~/Projects`, `~/Research`, `~/Personal`
2. **Continuous Indexing**: The AI monitors enabled directories and:
   - Extracts key information from files
   - Updates context files with new knowledge
   - Maintains relationships between concepts
3. **Privacy Control**: Users have granular control over what's indexed and stored

#### Bidirectional Updates

The system maintains perfect synchronization:

- **User edits in app** → Context files update → AI knowledge refreshes
- **AI generates content** → Saved to file system → Context files update
- **File system changes** → Detected and indexed → Context files update

This creates a living, breathing knowledge base that evolves with every interaction.

#### Natural Language Query Processing

The AI uses advanced semantic understanding to:

- **Parse intent**: Understand what you're really asking for
- **Identify concepts**: Map your query to relevant context topics
- **Retrieve information**: Pull full context for identified concepts
- **Synthesize answers**: Combine information from multiple sources
- **Handle complexity**: Answer both simple questions and complex multi-step requests

#### Data Model

```typescript
interface ContextFile {
  id: string;
  concept: string;  // e.g., "friends", "projects", "research"
  metadata: {
    created: string;
    lastUpdated: string;
    version: number;
    relatedConcepts: string[];  // Links to other context files
  };
  summary: string;  // High-level description (used for retrieval)
  content: {
    structured: Record<string, any>;  // Key-value pairs (birthdays, dates, etc.)
    references: ContentReference[];  // Links to source files/conversations
    embeddings: number[];  // Semantic vector representation
  };
}

interface ContentReference {
  type: 'file' | 'conversation' | 'note' | 'external';
  path: string;
  excerpt?: string;
  relevance: number;  // How relevant this reference is to the concept
}

interface DirectoryConfig {
  path: string;
  enabled: boolean;
  watchMode: 'realtime' | 'scheduled' | 'manual';
  excludePatterns: string[];  // Files/dirs to ignore
  priority: number;  // Higher priority = more frequent indexing
}
```

#### Implementation Phases

**Phase 0: Foundation (3-4 weeks)**
- Context file schema and storage system
- Basic directory watching and file indexing
- Simple concept extraction from conversations
- Basic retrieval for single-concept queries
- Deliverable: AI can remember and retrieve information from enabled directories

**Phase 1: Advanced Retrieval (3-4 weeks)**
- Multi-concept query processing
- Semantic embedding generation and similarity search
- Context file refinement (AI updates its own context files)
- Cross-concept relationship mapping
- Deliverable: Complex queries work across multiple concepts

**Phase 2: Bidirectional Sync (2-3 weeks)**
- Real-time file system monitoring
- Automatic context updates on file changes
- UI edits trigger context updates
- Conflict resolution for concurrent changes
- Deliverable: Perfect sync between app, files, and context

**Phase 3: Natural Language Optimization (ongoing)**
- Advanced query understanding
- Context-aware response generation
- Proactive context suggestions
- Learning from user corrections
- Deliverable: Seamless natural language interaction

---

### 2) Tool Generation & Community Sharing

#### Vision

Codex doesn't just use tools—it creates them, refines them, and shares them. Every tool the AI develops to help you becomes part of a shared ecosystem that improves the experience for all users.

#### Core Concept: Self-Extending Capability

As the AI works with you, it identifies repetitive tasks and creates tools to automate them. These tools are:
- **Stored locally** in your tools file system
- **Shared globally** with the community
- **Continuously improved** through collective use

#### Tool Categories

**1. MCP Server Integrations**
- Connect to external applications (Slack, Notion, GitHub, etc.)
- Parse application data into context
- Enable bidirectional communication with apps
- Example: "Connect to my Notion workspace and index all my notes"

**2. File Type Handlers**
- Read and write specialized file formats
- Extract structured data from files
- Generate files in specific formats
- Example: "Parse this Excel file and extract all project deadlines"

**3. Application Integrators**
- Extract information from running applications
- Inject data into applications
- Automate workflows across apps
- Example: "Read my calendar and add all meetings to my context"

**4. Agentic Loop Tools**
- Tools created during task execution
- Reusable automation scripts
- Custom workflows for specific needs
- Example: "Create a tool that formats my code and runs tests"

#### Tool Sharing Ecosystem

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tool Ecosystem                                │
├─────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌──────────────┐         ┌──────────────┐                      │
│  │  User A      │         │  User B      │                      │
│  │              │         │              │                      │
│  │ Creates Tool │────────▶│ Discovers    │                      │
│  │ "notion-     │         │ & Uses Tool  │                      │
│  │  parser"     │         │              │                      │
│  └──────────────┘         └──────────────┘                      │
│         │                        │                               │
│         │                        │                               │
│         ▼                        ▼                               │
│  ┌──────────────────────────────────────────────┐              │
│  │         Shared Tool Repository                │              │
│  │                                                │              │
│  │  - Version control                            │              │
│  │  - Ratings & reviews                          │              │
│  │  - Usage statistics                           │              │
│  │  - Automatic improvements                    │              │
│  └──────────────────────────────────────────────┘              │
│                                                                   │
└─────────────────────────────────────────────────────────────────┘
```

#### Tool Development Workflow

1. **Identification**: AI recognizes a repetitive task or need
2. **Creation**: AI generates a tool to solve the problem
3. **Testing**: Tool is tested in the current context
4. **Storage**: Tool saved to local tools directory
5. **Sharing**: User can opt-in to share tool with community
6. **Evolution**: Tool improves through community usage and feedback

#### Tool Schema

```typescript
interface Tool {
  id: string;
  name: string;
  description: string;
  version: string;
  author: string;  // User ID or "system"
  category: 'mcp-server' | 'file-handler' | 'app-integrator' | 'workflow';
  
  // Tool definition
  definition: {
    type: 'function' | 'mcp-server' | 'script';
    implementation: string;  // Code or config
    dependencies: string[];
    inputs: ToolInput[];
    outputs: ToolOutput[];
  };
  
  // Metadata
  metadata: {
    createdAt: string;
    lastUpdated: string;
    usageCount: number;
    rating?: number;
    tags: string[];
    relatedTools: string[];  // Other tools this works with
  };
  
  // Sharing
  sharing: {
    isPublic: boolean;
    shareId?: string;
    downloadCount?: number;
    forks?: number;
  };
}

interface ToolInput {
  name: string;
  type: string;
  description: string;
  required: boolean;
  default?: any;
}

interface ToolOutput {
  name: string;
  type: string;
  description: string;
}
```

#### Tool Discovery & Integration

Users can:
- **Browse** community tools by category, rating, or popularity
- **Search** for tools by functionality or use case
- **Install** tools with one click
- **Fork** tools to customize for their needs
- **Rate & Review** tools to help the community

The AI can:
- **Suggest** relevant tools based on current task
- **Auto-install** tools when needed
- **Combine** multiple tools for complex workflows
- **Generate** new tools when existing ones don't fit

#### Implementation Phases

**Phase 0: Tool Infrastructure (3-4 weeks)**
- Tool storage system and schema
- Basic tool execution engine
- Tool creation API for AI
- Local tool management UI
- Deliverable: AI can create and use custom tools

**Phase 1: MCP Server Framework (3-4 weeks)**
- MCP server integration system
- Standard MCP tool templates
- Application connector framework
- Deliverable: Connect to external apps and extract data

**Phase 2: File Type Handlers (2-3 weeks)**
- Extensible file handler system
- Common format handlers (PDF, Excel, Word, etc.)
- Custom format registration
- Deliverable: Read/write any file type

**Phase 3: Sharing Platform (4-5 weeks)**
- Tool sharing backend
- Community tool browser
- Version control for shared tools
- Rating and review system
- Deliverable: Users can share and discover tools

**Phase 4: Advanced Tool Features (ongoing)**
- Tool composition and workflows
- Automatic tool optimization
- Tool marketplace with monetization
- AI-powered tool suggestions
- Deliverable: Self-improving tool ecosystem

---

### 3) UI/UX Improvements

#### Slash Command Discovery
- Display available slash commands in the UI
- Commands: `/init`, `/agent`, `/help`, etc.
- Implementation: Add command palette or autocomplete dropdown when user types `/`
- Show command descriptions and usage examples

#### Chat Panel Enhancements
- Better message formatting (markdown rendering)
- Copy button for code blocks
- Collapsible long messages
- Loading states during AI responses

---

### 3) CLI Tooling Enhancements

#### Subagent System
- Implement specialized subagents like Claude Code
- Agent types: code-reviewer, test-runner, documentation, refactor
- Allow spawning background agents for parallel tasks

#### Multi-Model Support
- Add support for multiple AI providers via API
- Models: OpenAI GPT, Claude, Gemini, local models (Ollama, LM Studio)
- Model selector in settings
- Per-workspace model preferences

---

### 4) Chat History & Persistence

#### Save Conversations
- Persist chat history to disk
- Store in workspace directory or central location
- Load previous conversations on app restart

#### Conversation Management
- List past conversations
- Search conversation history
- Export conversations to markdown

---

### 5) Novel Features (Future)

#### Voice Integration
- Custom voice interface (JARVIS-style)
- Voice-to-text for prompts
- Text-to-speech for responses
- Conversational task execution

#### Enhanced AGENTS.md
- Better template generation
- Project-specific agent customization
- Auto-detection of project type and conventions

---

## Competitive Positioning

### vs Traditional AI Assistants (ChatGPT, Claude, etc.)

**Traditional AI**: Each conversation is isolated. No memory between sessions. Limited context window.

**Codex**: Single continuous conversation with persistent memory. Context grows and improves over time. Never forgets important information.

### vs Memory-Enabled AI (Mem.ai, Notion AI, etc.)

**Other Solutions**: Memory is fragmented. Requires manual organization. Limited to specific platforms.

**Codex**: Self-organizing context system. AI maintains its own memory structure. Works across all your files and applications.

### vs Automation Tools (Zapier, IFTTT, etc.)

**Automation Platforms**: Pre-built integrations only. No AI-driven tool creation. Static workflows.

**Codex**: AI creates custom tools for your needs. Community-shared tool ecosystem. Tools evolve and improve.

### Our Unique Value Proposition

**"Your AI that remembers everything and learns to help you better—one conversation, infinite memory, unlimited tools."**
