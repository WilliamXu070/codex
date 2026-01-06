# Codex Project Summary

## Overview
Codex is a multi-component repo that ships a Rust-based agent core (`codex-rs`) with multiple frontends and servers. This workspace now includes an Electron desktop app (`apps/codex-electron`) that provides a Finder-style file browser and a Codex prompt panel backed by the app-server JSON-RPC API.

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
- App-server binary is built from `codex-rs/` and spawned directly to keep JSON-RPC output clean.
- App-server initialization handshake is required before `newConversation` and `sendUserMessage`.
- File list scrolling + chat debug stream are in place.

---

## Next Steps

### 1) Core Feature: Workspace → Compile Document Generation System

#### Vision

Instead of trying to edit documents live inside Word/PowerPoint/Excel (which requires complex plugins and has file-locking issues), we flip the paradigm:

**Users don't edit the final document. They edit structured source content + templates, then compile to output.**

Think of it like software development:
- **Source** = content backbone, research, bullet points, notes
- **Config** = template/style/structure rules
- **Compiler** = builds final document output (.docx, .pptx, .xlsx, Notion)

This gives users a stable mental model: "I'm editing the plan, not fighting the final formatting."

#### Why This Approach is Superior

| Problem with Live Editing | How "Workspace → Compile" Solves It |
|---------------------------|-------------------------------------|
| File locking when Word is open | We own the source files; Word only sees exports |
| Complex Word/PPT/Excel APIs | We generate files programmatically, full control |
| User fears AI "ruining" their doc | Users edit the plan, AI generates from plan |
| Platform-specific (COM, Add-ins) | Pure Electron + Node.js, cross-platform |
| Hard to support multiple formats | Same workspace model, different renderers |

#### Core Concept: Four-Stage Workflow

The app becomes a "Deliverable IDE" with four tabs/stages:

**Stage 1: Plan**
- User creates structured outline with AI assistance
- Sections and subsections with bullet points
- Each bullet can be tagged: "must include", "optional", "needs citation"
- AI can: restructure outline, expand bullets, suggest missing sections
- Key feature: "Lock" sections so AI can't change the backbone

**Stage 2: Research**
- Drop PDFs, URLs, notes into a sources vault
- AI extracts key points, quotes, data from sources
- Link sources to specific bullets ("this claim is backed by source X")
- Traceability: every claim references a source or is marked "opinion"

**Stage 3: Style**
- Visual template builder (or pick from presets)
- Define style tokens: H1, H2, Body, Caption, Quote, TableHeader
- Layout rules: margins, spacing, TOC, section numbering, citation format
- Templates are portable JSON, work across output formats

**Stage 4: Build**
- "Build Draft" (quick preview) or "Build Final" (polished output)
- Preview shows diff vs previous build
- Version history for all builds
- "Open in Word/PPT/Excel" button for final output
- Key feature: Pin sections before rebuild to prevent AI regeneration

#### Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Electron App                                │
├─────────────────────────────────────────────────────────────────┤
│  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌──────────┐           │
│  │  Plan    │ │ Research │ │  Style   │ │  Build   │  ← Tabs   │
│  └──────────┘ └──────────┘ └──────────┘ └──────────┘           │
├─────────────────────────────────────────────────────────────────┤
│                     Workspace Model (JSON)                      │
│  ┌─────────────┐ ┌─────────────┐ ┌─────────────┐               │
│  │   Outline   │ │   Sources   │ │  Template   │               │
│  │  + Bullets  │ │ + Citations │ │  + Styles   │               │
│  └─────────────┘ └─────────────┘ └─────────────┘               │
├─────────────────────────────────────────────────────────────────┤
│                        Compiler                                 │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐         │
│  │ Content Pass│ →  │ Layout Pass │ →  │  Renderer   │         │
│  │ (AI expand) │    │(apply style)│    │ (docx/pptx) │         │
│  └─────────────┘    └─────────────┘    └─────────────┘         │
├─────────────────────────────────────────────────────────────────┤
│  Output: Report.docx / Deck.pptx / Model.xlsx / Notion blocks  │
└─────────────────────────────────────────────────────────────────┘
```

#### Data Model (Workspace Schema)

```typescript
interface Workspace {
  id: string;
  name: string;
  outline: OutlineSection[];
  sources: Source[];
  template: Template;
  builds: Build[];
  settings: WorkspaceSettings;
}

interface OutlineSection {
  id: string;
  title: string;
  level: number;  // 1 = H1, 2 = H2, etc.
  bullets: Bullet[];
  children: OutlineSection[];
  status: 'free' | 'guided' | 'pinned';  // Lock level
}

interface Bullet {
  id: string;
  text: string;
  sourceRefs: string[];  // Links to Source.id
  tags: ('must-include' | 'optional' | 'needs-citation')[];
  expanded?: string;  // AI-generated prose for this bullet
}

interface Source {
  id: string;
  type: 'pdf' | 'url' | 'note' | 'quote';
  title: string;
  content: string;
  metadata: { author?: string; date?: string; url?: string };
}

interface Template {
  basePreset: string;  // 'business-report' | 'academic' | 'pitch-deck'
  styles: {
    heading1: { fontFamily: string; fontSize: number; color: string; bold: boolean };
    heading2: { fontFamily: string; fontSize: number; color: string; bold: boolean };
    body: { fontFamily: string; fontSize: number; lineHeight: number };
    caption: { fontFamily: string; fontSize: number; italic: boolean };
  };
  layout: {
    pageSize: 'letter' | 'a4';
    margins: { top: number; bottom: number; left: number; right: number };
    includeCoverPage: boolean;
    includeTOC: boolean;
    sectionNumbering: 'none' | 'numeric' | 'roman';
    citationStyle: 'apa' | 'mla' | 'chicago' | 'numeric';
  };
}

interface Build {
  id: string;
  timestamp: string;
  outputFormat: 'docx' | 'pptx' | 'xlsx' | 'notion' | 'pdf';
  outputPath: string;
  sections: { id: string; wasRegenerated: boolean }[];
}
```

#### Compiler Design (Two-Pass)

**Pass 1: Content Pass (AI-driven)**
- Input: Outline + Bullets + Sources
- AI expands bullets into prose paragraphs
- Ensures tone consistency across sections
- Inserts citation markers
- Respects locked/pinned sections (skips regeneration)
- Output: Intermediate Representation (IR)

**Pass 2: Layout Pass (Deterministic)**
- Input: IR + Template
- Applies style tokens to content
- Handles page layout, margins, spacing
- Generates final file using format-specific library
- Output: .docx / .pptx / .xlsx / .pdf / Notion blocks

#### Technology Stack for Renderers

| Format | Library | Notes |
|--------|---------|-------|
| Word (.docx) | `docx` npm package | Full control over styles, paragraphs, tables |
| PowerPoint (.pptx) | `pptxgenjs` | Slide masters, text boxes, charts |
| Excel (.xlsx) | `exceljs` | Sheets, cells, formulas, charts |
| PDF | `pdfkit` or `puppeteer` | Generate from HTML or direct |
| Notion | Notion API | Block-based, straightforward |

#### Section Lock Mechanism (Control Feature)

Three states for each section:
- **Pinned**: AI cannot rewrite, only grammar fixes if requested
- **Guided**: AI can rewrite prose but must keep bullet points intact
- **Free**: AI has full control, can restructure and rewrite

This solves the "I rebuild and it rewrites everything" fear.

#### UI Integration with Existing App

```
┌─────────────────────────────────────────────────────────────────┐
│  Top Bar: [File Browser] [Workspace: Project Report ▼]         │
├─────────────────────────────────────────────────────────────────┤
│  ┌─ Sidebar ─┐  ┌─ Main Area ───────────────────────────────┐  │
│  │           │  │                                           │  │
│  │ Plan      │  │   [Current Tab Content]                   │  │
│  │ Research  │  │                                           │  │
│  │ Style     │  │   - Outline editor (Plan)                 │  │
│  │ Build     │  │   - Sources list (Research)               │  │
│  │           │  │   - Template builder (Style)              │  │
│  │ ───────── │  │   - Preview + export (Build)              │  │
│  │ Chat      │  │                                           │  │
│  │           │  │                                           │  │
│  └───────────┘  └───────────────────────────────────────────┘  │
│                                                                 │
│  ┌─ Bottom Panel (collapsible) ─────────────────────────────┐  │
│  │ AI Chat: Context-aware based on current tab              │  │
│  │ Plan: "Restructure outline" / "Add section about X"      │  │
│  │ Research: "Summarize this PDF" / "Find sources for Y"    │  │
│  │ Build: "Regenerate section 3" / "Make tone more formal"  │  │
│  └──────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────┘
```

#### Implementation Phases

**Phase 0: Foundation (2-3 weeks)**
- Define workspace JSON schema with Zod validation
- Create/save/load workspace files (.codex-workspace.json)
- Basic Plan tab UI (outline tree + bullet editor)
- Integrate with existing chat for AI bullet expansion
- Basic DOCX export (no styling)
- Deliverable: User can create outline, expand with AI, export basic Word doc

**Phase 1: Templates + Styled Export (2-3 weeks)**
- Template schema + 5 presets (Business Report, Academic, Pitch Deck, Meeting Notes, Project Proposal)
- Style tab UI (preset picker + basic tweaks)
- Styled DOCX export with proper formatting
- Build versioning and history
- Deliverable: User can pick template, export styled Word doc

**Phase 2: Research + Citations (2-3 weeks)**
- Research tab UI with drag-drop zone
- PDF text extraction using pdf-parse
- Source ↔ bullet linking UI
- Citation insertion in exports
- Deliverable: Full Plan → Research → Build workflow

**Phase 3: PowerPoint + Excel Renderers (2-3 weeks)**
- PPT renderer using pptxgenjs
- Excel renderer using exceljs
- Format-specific template options
- Deliverable: Same workspace can export to Word, PPT, or Excel

**Phase 4: Polish + Advanced (ongoing)**
- Full lock/pin mechanism for sections
- Diff view between builds
- Notion export
- Visual template designer (WYSIWYG)
- "Deliverable bundles" (report + deck + spreadsheet from same workspace)

#### Competitive Positioning

**vs Microsoft Copilot**: Copilot helps write inside a doc. We help plan + structure + cite + template + generate. We produce consistent outputs across Word/PPT/Excel.

**vs Google Docs AI**: Same limitation - in-doc assistance. We own the full workflow.

**Our product is**: "Deliverable Generator" not "Writing Helper"

---

### 2) UI/UX Improvements

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
