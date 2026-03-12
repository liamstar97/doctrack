---
name: doctrack
description: >
  Maintains persistent codebase knowledge across sessions through structured feature and component
  documentation. Use this skill whenever you have just made meaningful code changes (new features,
  modified components, refactoring, bug fixes) to update the project's documentation. Also use it
  when the user asks to document code, update docs, sync documentation, initialize documentation
  for an existing project, or when you want to understand the existing codebase structure at the
  start of a session. This skill should be used proactively after any significant code modification
  — don't wait for the user to ask. If you changed code, update the docs. Think of it as your
  long-term memory system: read before working, write after changing. Also use this when a user
  says "doctrack init", "initialize docs", "document this project", or wants to bootstrap
  documentation for a codebase that has no .claude_docs/ directory yet.
---

# Doctrack — Persistent Codebase Knowledge

You maintain two documentation trees that serve as your long-term memory about the codebase:

- **`.claude_docs/`** — Your internal knowledge base. Dense, structured, optimized for you to quickly understand the codebase in future sessions.
- **`docs/`** — Human-readable documentation. Clean, polished, organized for developers.

## When to use this

**After making code changes**: Any time you add a feature, modify a component, refactor code, or fix a non-trivial bug, update the relevant documentation before finishing your response.

**At the start of a session**: When you begin working on a codebase that has a `.claude_docs/` directory, read `.claude_docs/index.md` first to orient yourself. This is your memory from previous sessions — use it.

**When the user asks**: If the user says anything about documenting, updating docs, syncing docs, or generating documentation.

**To initialize a project**: When the user says "doctrack init" or asks to document an existing project that has no `.claude_docs/` yet. See the **Project Initialization** section below.

**Do NOT use this for**: Trivial formatting changes, comment-only edits, or when you're just reading/exploring code without modifying it.

## Directory structure

```
project_root/
├── .claude_docs/                          # Your internal knowledge base
│   ├── index.md                           # Master index — always read this first
│   ├── references/                        # Reference documents
│   │   ├── imported/                      # Existing project docs imported here
│   │   └── user/                          # User-provided reference materials
│   ├── {feature-name}/
│   │   ├── feature.md                     # Feature-level overview
│   │   └── components/
│   │       ├── {component-name}.md        # Component-level detail
│   │       └── ...
│   └── ...
└── docs/                                  # Human-readable documentation
    ├── legacy/                            # Pre-existing docs moved here during init
    ├── architecture.md                    # System architecture overview
    ├── {topic}.md                         # Topic-based docs
    └── ...
```

### Reference documents (`.claude_docs/references/`)

This directory holds supplementary documentation that you should consult when working on the codebase:

- **`references/imported/`** — Existing project documentation that was present before doctrack was initialized (e.g., original README files, wiki exports, design docs, ADRs). During init, copy these here for your internal use. If the originals lived in a `docs/` directory, move them to `docs/legacy/` so they're preserved for humans but out of the way of new doctrack-generated docs. This avoids duplication — `references/imported/` is your copy, `docs/legacy/` is the human-accessible archive.

- **`references/user/`** — Documents the user has added for you to reference. These might be API docs for external services, design specs, style guides, compliance requirements, or anything else relevant to the project. Users can drop files here at any time.

When starting a session, after reading `index.md`, check if `references/` exists and scan its contents. These documents provide context that may not be captured in feature/component docs — design rationale, external API contracts, business requirements, etc. Reference them in your feature docs when relevant (e.g., "See `references/user/stripe-api-guide.md` for webhook signature verification details").

## Step-by-step workflow

### 1. Bootstrap (first time only)

If `.claude_docs/` doesn't exist yet and you're just making a single change (not initializing the whole project), create a minimal structure.

**Determine the right location for `.claude_docs/`:**
- **Standard project**: Create `.claude_docs/` at the project root.
- **Monorepo**: If you're working inside a package (e.g., `packages/api/src/...`), create `.claude_docs/` inside that package (e.g., `packages/api/.claude_docs/`). Also create the root `.claude_docs/index.md` if it doesn't exist, with a basic package map. Check for monorepo indicators: `workspaces` in root `package.json`, `pnpm-workspace.yaml`, `lerna.json`, `turbo.json`, `nx.json`.

```
.claude_docs/
└── index.md
```

Initialize `index.md` with this template:

```markdown
# Project Documentation Index

> Auto-maintained by doctrack. Last updated: YYYY-MM-DD

## Features

| Feature | Path | Description | Status | Last Updated |
|---------|------|-------------|--------|--------------|

## File Registry

| Source File | Feature | Component | Role |
|------------|---------|-----------|------|
```

Also create `docs/` if it doesn't exist (co-located with the `.claude_docs/` — at package level for monorepos, at root for standard projects).

For full project initialization (documenting an entire existing codebase), see the **Project Initialization** section below instead.

### 2. Read existing documentation

**Always read `.claude_docs/index.md` first.** This tells you what's already documented and prevents duplicate or conflicting documentation.

**In a monorepo**: Find the nearest `.claude_docs/` by walking up from your current working directory. If you're inside a package, read that package's `.claude_docs/index.md` first, then the root `.claude_docs/index.md` for cross-package context. If you're at the repo root, read the root index to understand which packages exist and their relationships.

Then check if `.claude_docs/references/` exists — scan its contents for any user-provided or imported docs that may be relevant to your current task.

Then read the specific feature/component docs relevant to whatever code you just changed. The index tells you which docs exist and where they are.

### 3. Update internal docs (`.claude_docs/`)

Be surgical — only update docs for code that actually changed. Don't rewrite everything.

#### Feature docs: `.claude_docs/{feature}/feature.md`

Use this template:

```markdown
---
feature: feature-name
type: feature
files:
  - src/path/to/file.ts
  - src/path/to/other.ts
last_updated: YYYY-MM-DD
status: active
---

# Feature Name

## Purpose
What this feature does and why it exists. Be specific — your future self needs to
understand this without reading the code.

## Architecture
How the feature is structured. Key design decisions and why they were made.
Data flow, state management approach, important patterns.

## Key Files
- `src/path/to/file.ts` — Main entry point, handles X
- `src/path/to/other.ts` — Utility functions for Y

## Dependencies
- **Internal**: auth module, database layer
- **External**: express, lodash

## API Surface
Key exports, endpoints, or interfaces that other parts of the codebase use.

## Notes
Anything important for future sessions: gotchas, technical debt, planned changes,
non-obvious behavior.
```

#### Component docs: `.claude_docs/{feature}/components/{component}.md`

```markdown
---
feature: parent-feature-name
type: component
files:
  - src/path/to/component.ts
last_updated: YYYY-MM-DD
status: active
---

# Component Name

## Responsibility
Single-sentence description of what this component does.

## Key Files
- `src/path/to/component.ts:15-80` — Core logic
- `src/path/to/types.ts:5-20` — Type definitions

## Public API
```typescript
// Key exports, function signatures, or interface definitions
```

## Internal Logic
How it works internally. Important algorithms, state transitions, data transformations.
Be dense — this is for you, not humans.

## Relationships
- **Used by**: list of components/features that depend on this
- **Depends on**: list of components/features this uses

## Known Issues
- Any bugs, technical debt, or TODOs
```

#### Update the index

After creating or modifying any feature/component docs, update `.claude_docs/index.md`:
- Add new rows to the Features table
- Add new rows to the File Registry
- Update the "Last Updated" timestamp

### 4. Update human-readable docs (`docs/` and `README.md`)

After updating internal docs, update the human-facing documentation. These docs are meant for developers — they should be descriptive, visual, and immediately useful. Think of them as the "front page" of the project that someone reads before diving into code.

#### README files

**Every project and sub-project must have a `README.md` at its root.** The README is the front page for humans and git repositories. It should include:
- Project name and a concise description of what it does
- Tech stack and key dependencies
- Quick start: how to build, run, and test the project
- Links to detailed docs in `docs/` for deeper reading
- For monorepos: the root README maps all sub-projects and links to each sub-project's README

#### `docs/` directory

Create descriptive, feature-focused documents — not just a single architecture file. Each major feature or subsystem should get its own doc. These should be higher level than `.claude_docs/` but still descriptive enough that a developer can understand how features work, what mechanisms are involved, and what technologies are used.

**What to include:**
- **Architecture overview** (`docs/architecture.md`): High-level system design, how features connect, data flow. Use ASCII art generously for visual explanations — flow diagrams, sequence diagrams, state machines, data flow charts, component relationship maps. These make docs immediately scannable and help developers build mental models fast. The only ASCII art to avoid is directory/file tree listings (developers can search the codebase themselves).
- **Feature docs** (`docs/{feature}.md`): One per major feature. Explain what it does, how it works, key mechanisms and patterns, and important configuration. Include code examples only when they genuinely clarify something that prose cannot (e.g., a non-obvious API usage pattern or a tricky integration point). Use ASCII art diagrams liberally — flow charts, sequence diagrams, state transitions, data pipelines. Tables and structured sections help too.
- **API docs** (`docs/api.md`): If there are APIs, document endpoints and usage in human-readable form.
- **OpenAPI spec** (`docs/openapi.yaml`): If the project exposes REST APIs, generate or update an OpenAPI 3.0+ spec. This is more useful to developers than prose — it can be imported into Postman, used for client generation, and rendered by Swagger UI. Include all endpoints, request/response schemas, auth requirements, and error responses. Keep it in sync with the actual routes.
- **Build, run, and test** (`docs/development.md` or in README): How to set up the dev environment, build the project, run it locally, and run tests (if tests exist). Include actual commands.
- **Glossary** (`docs/glossary.md`): If the project has domain-specific terminology.

**Style guidelines:**
- Polished and well-written — humans will read these
- Use clear headings, tables, and ASCII art diagrams liberally (flow charts, sequence diagrams, state machines, data flow, component maps)
- Include code examples sparingly — only when they genuinely clarify something
- Do NOT include ASCII directory/file tree listings — that info is easily discoverable via the codebase
- Maintain a consistent tone across all docs

Only create/update human docs that are relevant to the changes made. Don't create empty placeholder docs.

When updating API endpoints, always update both `docs/api.md` and `docs/openapi.yaml` together. If the project doesn't have REST APIs (e.g., it's a library or CLI tool), skip the OpenAPI spec.

### 5. Deprecation handling

When code is removed or replaced:
- Set `status: deprecated` in the frontmatter
- Add a deprecation note explaining what replaced it and when
- Do NOT delete the doc — it serves as historical context
- Update the index to reflect the deprecated status

## Naming conventions

- **Feature directories**: lowercase, kebab-case (e.g., `user-authentication`, `payment-processing`)
- **Component files**: lowercase, kebab-case matching the component name (e.g., `token-validator.md`, `payment-gateway.md`)
- **Human docs**: lowercase, kebab-case, descriptive (e.g., `getting-started.md`, `api-reference.md`)

## Important principles

1. **Read before writing.** Always check what docs already exist before creating new ones. Prevent duplicates.

2. **Paths are relative.** Always use paths relative to the project root. Never absolute paths.

3. **Dense internal docs.** Your `.claude_docs/` files are for you — pack them with information. Include the "why" behind decisions, not just the "what."

4. **Clean human docs.** The `docs/` files are for developers — keep them readable, well-organized, and free of internal implementation noise.

5. **Incremental updates.** Don't rewrite entire docs when only one section changed. Edit surgically.

6. **Timestamp everything.** Always update `last_updated` in frontmatter when modifying a doc. This helps you prioritize which docs might be stale.

7. **Feature boundaries matter.** Think carefully about what constitutes a "feature" vs a "component." A feature is a cohesive unit of functionality (e.g., "authentication", "search"). A component is a distinct part within a feature (e.g., "token-validator", "search-indexer").

---

## Project Initialization

Use this when a codebase already has significant code but no `.claude_docs/` directory. The user might say "doctrack init", "initialize docs", "document this project", or similar.

This is fundamentally different from the incremental workflow above — you're documenting an entire existing codebase rather than updating docs after a single change.

### Init workflow

#### Phase 1: Discover the project structure

1. **Read config files** to understand the tech stack:
   - `package.json`, `Cargo.toml`, `pyproject.toml`, `go.mod`, etc.
   - Framework config files (next.config.js, vite.config.ts, etc.)
   - Look at the dependency list — it reveals what the project does
   - **Check for monorepo indicators**: `workspaces` in `package.json`, `pnpm-workspace.yaml`, `lerna.json`, `turbo.json`, `nx.json`, or a `packages/`/`apps/`/`services/` directory with multiple sub-projects. If this is a monorepo, follow the **Init for monorepos** section below instead of continuing with Phases 2-4 directly.

2. **Map the directory tree** — use glob patterns to understand the layout:
   - `src/**/*.{ts,js,py,go,rs}` (or whatever the language is)
   - Look for natural groupings: `src/features/`, `src/modules/`, `src/routes/`, `src/components/`, etc.
   - Note test directories, config directories, scripts

3. **Import existing documentation** — look for any pre-existing docs in the project:
   - README files (`README.md`, `packages/*/README.md`)
   - Doc directories (`docs/`, `documentation/`, `wiki/`)
   - Architecture Decision Records (`adr/`, `decisions/`)
   - Design docs, API specs, runbooks
   - **For standard projects**: Copy these into `.claude_docs/references/imported/`, preserving filenames.
   - **For monorepos**: Import root-level docs (root README, root docs/) into the root `.claude_docs/references/imported/`. Import package-specific docs (e.g., `packages/api/README.md`, `packages/api/docs/`) into that package's `.claude_docs/references/imported/`. This keeps references co-located with the code they describe.
   - **Handle pre-existing `docs/` directories**: If a `docs/` directory already exists with content, move (not copy) its contents into `docs/legacy/` to preserve them while making room for doctrack-generated documentation. This prevents duplication — the originals live in `docs/legacy/` for human reference, and copies go into `.claude_docs/references/imported/` for your internal use. Do not leave pre-existing docs in both `docs/` and `references/imported/`.
   - These are your source material for understanding the project and should be referenced from feature docs where relevant.

4. **Identify feature boundaries** — this is the critical thinking step. Look for:
   - Directory-based groupings (e.g., `src/auth/`, `src/payments/`)
   - Route files that reveal API structure
   - Domain models that reveal business concepts
   - If the project is flat (no clear directories), group by domain concept based on file names and imports

#### Phase 2: Document features in parallel

For large projects, use subagents to document features in parallel. Each subagent should:

1. Receive: the feature name, the list of files belonging to that feature, and the doctrack templates
2. Read all source files for that feature
3. Write the feature doc AND component docs following the templates
4. Return: the paths of docs created and the file registry entries

**Component docs are not optional.** A feature with multiple source files should have component docs — one for each distinct logical unit (a service class, a middleware, a data model, a utility module, etc.). The feature doc describes the big picture; component docs describe the internals of each piece. If a feature has only one source file, a component doc is not needed. But features with 2+ files almost always warrant components.

Spawn one subagent per feature. Give each subagent this context:
```
You are documenting the "{feature-name}" feature for doctrack.

Files to analyze:
- {list of source files}

Other features in this project (for cross-referencing):
- {list of other feature names and their file paths}

Write these docs following the doctrack templates:
1. .claude_docs/{feature}/feature.md (the feature overview)
2. .claude_docs/{feature}/components/{component}.md (one per logical component)

You MUST create component docs for features with multiple source files. Each distinct
class, service, middleware, model, or utility module should get its own component doc
with type: component in the frontmatter. The feature doc covers the big picture;
component docs cover the internals of each piece.

Use the standard frontmatter format. Be thorough — this is the initial documentation
that future sessions will rely on. Focus on architecture, key decisions, file roles,
dependencies, and anything non-obvious about how the code works.

IMPORTANT: Trace imports and function calls to identify cross-feature dependencies.
In the Dependencies section, explicitly list every other feature this one imports from
or depends on. In component docs, populate the "Used by" and "Depends on" fields in
the Relationships section. These cross-references are critical — they help future
sessions understand how changes to one feature ripple through others.
```

#### Phase 3: Build the index, README, and human docs

After all feature docs are written:

1. **Build `.claude_docs/index.md`** — aggregate all features, components, and file mappings into the master index. The index does not need YAML frontmatter — it's a table-of-contents file, not a feature/component doc.
2. **Write `README.md`** at the project root — the front page for humans and git repositories. Include project name, description, tech stack, how to build/run/test, and links to docs. For monorepos, also write a README for each sub-project.
3. **Write `docs/architecture.md`** — a high-level overview of how the system fits together, with visual descriptions of data flow and feature interactions. Avoid ASCII directory trees — focus on concepts and relationships.
4. **Write `docs/{feature}.md`** for each major feature — descriptive documents explaining how features work, their mechanisms, patterns, and technologies. Use diagrams, tables, and visual descriptions. Include code examples only when they genuinely clarify something prose cannot. These should be higher level than `.claude_docs/` feature docs but still substantive enough for a developer to understand the feature without reading source code.
5. **Write `docs/openapi.yaml`** — if the project has REST API endpoints, generate an OpenAPI 3.0+ spec covering all routes, request/response schemas, authentication, and error responses
6. **Write `docs/api.md`** — human-readable API reference if the project has APIs
7. **Write `docs/development.md`** — build, run, and test instructions with actual commands (if applicable)
8. **Write `docs/glossary.md`** if the project has domain-specific terminology

#### Phase 3.5: Verify cross-references

After all feature docs exist, do a cross-reference pass:

1. **Check every feature's Dependencies section** — does it list all the other features it imports from? Read the source files' import statements to verify.
2. **Check component Relationships sections** — are "Used by" and "Depends on" populated? A component that imports from another feature's files should list that dependency.
3. **Fill in gaps** — if a feature imports from another feature but doesn't mention it in Dependencies, add it. This is the most common gap in init documentation and it matters because future sessions use these cross-references to understand change impact.

#### Phase 4: Verify completeness

Check that every source file in the project appears in the File Registry. Files that don't belong to any feature likely indicate:
- A feature you missed (create docs for it)
- Utility/shared code (document as a "shared" or "common" feature)
- Dead code (note it in the index)

### Init for monorepos

For monorepos with multiple packages/apps, use **separate `.claude_docs/` directories per package** with a lightweight root-level coordination file. This matches how monorepo work actually happens — scoped to a package — while still capturing cross-package relationships.

**Detecting monorepos:** Look for `workspaces` in `package.json`, `pnpm-workspace.yaml`, `lerna.json`, Turborepo config (`turbo.json`), Nx config (`nx.json`), or multiple `go.mod` files. Also check for a `packages/`, `apps/`, or `services/` directory containing multiple sub-projects with their own config files.

#### Monorepo directory structure

```
monorepo/
├── .claude_docs/
│   └── index.md                       # Root coordination: package map, cross-package deps, shared conventions
├── packages/
│   ├── api/
│   │   ├── .claude_docs/              # Full doctrack structure for api
│   │   │   ├── index.md
│   │   │   ├── references/
│   │   │   ├── {feature}/
│   │   │   │   ├── feature.md
│   │   │   │   └── components/
│   │   │   └── ...
│   │   ├── docs/                      # Human-readable docs for api
│   │   └── src/
│   ├── web/
│   │   ├── .claude_docs/              # Full doctrack structure for web
│   │   │   ├── index.md
│   │   │   └── ...
│   │   ├── docs/
│   │   └── src/
│   └── shared/
│       ├── .claude_docs/              # Full doctrack structure for shared
│       │   ├── index.md
│       │   └── ...
│       ├── docs/
│       └── src/
├── docs/                              # Root-level human docs (architecture overview, getting started)
└── ...
```

#### Root `.claude_docs/index.md` for monorepos

The root index is a coordination file, not a full documentation index. It should contain:

```markdown
# Monorepo Documentation Index

> Auto-maintained by doctrack. Last updated: YYYY-MM-DD

## Packages

| Package | Path | Description | Tech Stack |
|---------|------|-------------|------------|
| @repo/api | packages/api | REST API server | Express, TypeScript |
| @repo/web | packages/web | Frontend app | React, TypeScript |
| @repo/shared | packages/shared | Shared types and utils | TypeScript |

## Cross-Package Dependencies

| Package | Depends On | Relationship |
|---------|-----------|--------------|
| @repo/api | @repo/shared | Types, validation schemas, constants |
| @repo/web | @repo/shared | Types, constants |
| @repo/web | @repo/api | HTTP API consumer |

## Shared Conventions
- [List monorepo-wide patterns, build conventions, shared tooling, etc.]
```

#### Per-package `.claude_docs/`

Each package gets the full doctrack structure — its own `index.md`, features, components, file registry, and `references/` directory. Document each package as if it were a standalone project, but note cross-package dependencies explicitly in feature docs.

In each package's feature docs, reference other packages by name:
```markdown
## Dependencies
- **Internal**: auth-middleware, database layer
- **Cross-package**: `@repo/shared` (types, validation schemas)
- **External**: express, cors
```

#### Monorepo init workflow

1. **Detect monorepo structure** — identify packages/apps and their boundaries
2. **Create root `.claude_docs/index.md`** — map all packages, their relationships, and shared conventions
3. **Init each package independently** — run the standard Phase 1-4 init workflow within each package, creating `packages/{name}/.claude_docs/` and `packages/{name}/docs/`
4. **Write root `docs/architecture.md`** — high-level overview of the entire monorepo, how packages fit together, deployment topology
5. **Cross-reference pass** — ensure each package's docs correctly reference their cross-package dependencies

Use subagents to init multiple packages in parallel — each package is independent and won't conflict.

#### Finding docs in a monorepo

When starting work in a monorepo, determine your scope:

- **Working on a specific package**: Read that package's `.claude_docs/index.md` first, then the root `.claude_docs/index.md` for cross-package context
- **Working across packages**: Read the root `.claude_docs/index.md` first, then the relevant package indexes
- **Navigating from a file**: Find the nearest `.claude_docs/` by walking up from the file's directory — this gives you the right package context

---

## Working with teams (multi-agent / worktrees)

When other agents or team skills use doctrack — especially in git worktree setups where multiple agents work on different branches simultaneously — there are coordination challenges. This section helps teams use doctrack effectively.

### For team orchestrators (the skill that manages teams)

If you're building a skill that spawns team members in worktrees, include these instructions when assigning tasks:

```
Documentation requirements:
- Read .claude_docs/index.md before starting work to understand the codebase
- After completing your code changes, update .claude_docs/ for any features/components you modified
- Only update docs for code YOU changed — don't touch docs for other features
- Use the doctrack templates (feature.md, components/*.md) with standard frontmatter
```

### How doctrack works across worktrees

Each worktree gets its own copy of `.claude_docs/`. When branches merge back:

1. **Doc updates for different features** merge cleanly (different files, no conflicts)
2. **Index.md will conflict** if multiple agents added entries — this is expected and easy to resolve (just combine the table rows)
3. **Same-feature doc updates** will conflict if two agents modified the same feature — the merge resolver should keep the more recent/comprehensive version

### Minimizing merge conflicts

Doctrack is designed to be merge-friendly:

- **One file per component** means agents working on different components never conflict
- **Feature-level docs** only conflict if two agents modify the same feature (uncommon in well-scoped tasks)
- **The index** is the most likely conflict point. To minimize this:
  - Each agent should only ADD rows, never rewrite the whole table
  - Use append-style edits rather than rewriting index.md from scratch

### What team members should do

When an agent starts work in a worktree:

1. **Read `.claude_docs/index.md`** — understand what's documented and what your task touches
2. **Read relevant feature/component docs** — get up to speed on the area you'll be working in
3. **Do your code work**
4. **Update docs for what you changed** — create or edit feature/component docs as needed
5. **Append new entries to index.md** — don't rewrite it, just add your new rows at the bottom of each table

### Post-merge reconciliation

After merging worktree branches back, the orchestrator (or a dedicated reconciliation step) should:

1. **Resolve any index.md conflicts** — combine all table entries, deduplicate
2. **Check for stale docs** — if a merge deleted code, mark related docs as deprecated
3. **Update timestamps** — set `last_updated` to the merge date for any docs that were conflict-resolved
4. **Verify the File Registry** — ensure all source files are still mapped correctly after the merge
