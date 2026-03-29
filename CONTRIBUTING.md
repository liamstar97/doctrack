# Contributing to Doctrack

Thanks for your interest in improving Doctrack! This guide covers how the skill works and how to make changes.

## Repository structure

```
doctrack/
├── doctrack/
│   └── SKILL.md              # The skill — knowledge graph schema and workflows
├── doctrack.skill             # Packaged skill file (zip archive)
├── doctrack-workspace/        # Test workspace (not shipped)
│   ├── evals/
│   │   └── evals.json         # Test case definitions
│   └── iteration-*/           # Test run results
├── README.md
├── CONTRIBUTING.md
├── LICENSE
└── .mcp.json                  # MCP config for testing (not shipped)
```

## Architecture

Doctrack is a **two-skill system**:

1. **Doctrack** (`doctrack/SKILL.md`) — Defines the knowledge graph: what notes to create, what structure, what frontmatter, what wikilinks. This is what you edit.
2. **Obsidian skill** (`bitbonsai/mcpvault`) — Handles vault I/O via MCP tools. This is an external dependency.

Doctrack never calls MCP tools directly in its instructions — it describes **what** to do ("write a feature note", "tag it", "search for existing notes") and the obsidian skill figures out **how**.

## Making changes

### 1. Edit SKILL.md

All doctrack logic lives in `doctrack/SKILL.md`. Key sections:

| Section | What it controls |
|---------|-----------------|
| **Knowledge graph structure** | Node types, wikilink patterns |
| **Tag taxonomy** | How notes are categorized |
| **Session init** | How Claude orients at session start |
| **Note templates** | Frontmatter and content structure for each note type |
| **Project initialization** | The full init workflow |
| **Version tracking** | Migration paths between versions |

**Tips:**
- Explain "why" not just "what" — Claude follows instructions better with reasoning
- Use templates and examples — they produce consistent output
- Test on real codebases, not just mock projects

### 2. Test your changes

#### Quick test (manual)

```bash
# Install your modified skill
claude install-skill ./doctrack.skill

# In a project directory:
# "doctrack init"
```

#### Full eval suite

If you have the skill-creator skill, run the eval framework:

1. Test cases are in `doctrack-workspace/evals/evals.json`
2. Spawn test runs with and without the skill
3. Grade assertions against outputs
4. Review in the eval viewer

#### Live MCP test

For end-to-end testing with the actual MCP tools:

1. Install mcpvault: `npx skills add bitbonsai/mcpvault --yes`
2. Create a test vault and configure `.mcp.json`
3. Run `doctrack init` and verify notes appear in the vault
4. Open the vault in Obsidian to check the graph view

### 3. Repackage

```bash
zip -r doctrack.skill doctrack/
```

Or with the skill-creator:

```bash
claude -p "package the skill at ./doctrack"
```

### 4. Test the package

```bash
claude install-skill ./doctrack.skill
```

## Areas for contribution

### Knowledge graph
- New node types (e.g., "risk" notes for security concerns, "todo" for planned work)
- Better concept detection during init
- Smarter decision extraction from code comments and commit messages

### Monorepo support
- Additional detection patterns (Bazel, Pants, custom layouts)
- Better cross-package dependency tracking

### Language/framework coverage
- Framework-specific documentation patterns
- Language-specific conventions for different tech stacks

### Mermaid diagrams
- Better diagram templates for specific patterns
- Auto-generation of dependency graphs from import analysis

### Migration
- Smoother v1/v2 → v3 migration
- Migration from other documentation systems (JSDoc, Sphinx, etc.)

## Submitting changes

1. Fork the repository
2. Create a branch
3. Edit `SKILL.md`
4. Test with at least one real project
5. Repackage the `.skill` file
6. Open a PR with what you changed, why, and how you tested it
