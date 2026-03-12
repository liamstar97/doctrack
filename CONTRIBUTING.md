# Contributing to Doctrack

Thanks for your interest in improving Doctrack! This guide covers how the skill works internally and how to make changes.

## Repository structure

```
doctrack/
├── doctrack/
│   └── SKILL.md              # The skill itself — all instructions live here
├── doctrack.skill             # Packaged skill file (zip archive)
├── doctrack-workspace/        # Test workspace (not included in the skill package)
│   ├── evals/
│   │   └── evals.json         # Test case definitions and assertions
│   ├── mock-projects/         # Mock projects for testing
│   └── iteration-*/           # Test run results by iteration
├── README.md
├── CONTRIBUTING.md
└── LICENSE
```

## How the skill works

Doctrack is a single `SKILL.md` file with YAML frontmatter and markdown instructions. When installed as a Claude Code skill, Claude reads these instructions and follows them to manage documentation.

### Key sections of SKILL.md

| Section | What it controls |
|---------|-----------------|
| **Frontmatter** (`name`, `description`) | When the skill triggers — the description is the primary trigger mechanism |
| **Directory structure** | Where docs live and how they're organized |
| **Step-by-step workflow** | The incremental update flow: bootstrap → read → update internal docs → update human docs |
| **Templates** | Feature doc and component doc templates with frontmatter format |
| **Project Initialization** | The `doctrack init` workflow: discover → document → index → verify |
| **Init for monorepos** | Monorepo detection, per-project `.claude_docs/`, root coordination |
| **Working with teams** | Multi-agent and worktree coordination |

### How triggering works

The `description` field in the frontmatter determines when Claude invokes the skill. Claude sees all installed skill descriptions in its context and decides which to use based on the user's request. The description should be specific enough to trigger reliably but broad enough to cover edge cases.

## Making changes

### 1. Edit SKILL.md

All skill logic lives in `doctrack/SKILL.md`. Edit this file directly. Key things to keep in mind:

- **Explain the "why"** — Claude follows instructions better when it understands the reasoning, not just the rules. Prefer explaining motivation over rigid directives.
- **Be specific about output format** — Templates and examples help Claude produce consistent results.
- **Test with real codebases** — Mock projects are useful for automated testing, but real-world codebases reveal edge cases.

### 2. Test your changes

Doctrack uses the [skill-creator](https://github.com/anthropics/claude-code) eval framework for testing. Test cases live in `doctrack-workspace/evals/evals.json`.

#### Running a test manually

The simplest way to test is to run `doctrack init` on a real project with your modified skill:

```bash
# Point Claude at your modified skill
claude --skill ./doctrack/SKILL.md

# Then in the conversation:
# "doctrack init"
```

#### Running evals with skill-creator

If you have the skill-creator skill installed, you can run the full eval suite:

1. Define test cases in `doctrack-workspace/evals/evals.json`
2. Use the skill-creator to spawn test runs with and without the skill
3. Grade assertions against the outputs
4. Review results in the eval viewer

See `doctrack-workspace/evals/evals.json` for the existing test cases and assertion format.

#### Test case structure

```json
{
  "id": 3,
  "prompt": "doctrack init",
  "expected_output": "Description of what should be produced",
  "assertions": [
    {"id": "root-index-created", "text": "Root .claude_docs/index.md exists with a Packages table"}
  ]
}
```

### 3. Repackage the skill

After making changes, repackage the `.skill` file:

```bash
# From the repository root
zip -r doctrack.skill doctrack/
```

Or if you have the skill-creator installed:

```bash
claude -p "package the skill at ./doctrack"
```

The `.skill` file is just a zip archive containing the `doctrack/` directory.

### 4. Test the packaged skill

Install your packaged skill locally and test it:

```bash
claude install-skill ./doctrack.skill
```

## Areas for contribution

### Documentation quality

- Improve the templates for feature docs, component docs, or human-readable docs
- Add support for additional documentation formats (e.g., Mermaid diagrams, MDX)
- Better handling of specific frameworks or project types

### Monorepo support

- Additional monorepo detection patterns (Bazel, Pants, custom workspace layouts)
- Better cross-package dependency tracking
- Handling of polyglot monorepos

### Language/framework coverage

- Framework-specific documentation patterns (Next.js, Django, Rails, etc.)
- Language-specific conventions for different tech stacks
- Better detection of project types and tech stacks

### Init workflow

- Smarter feature boundary detection
- Better handling of large codebases (file count limits, prioritization)
- Improved parallelization strategy for init

### Team workflows

- Better merge conflict resolution strategies
- Integration with CI/CD for doc validation
- Automated staleness detection

## Code of conduct

Be kind, constructive, and respectful. We're all here to make better tools.

## Submitting changes

1. Fork the repository
2. Create a branch for your change
3. Make your edits to `SKILL.md`
4. Test with at least one real project
5. Repackage the `.skill` file
6. Open a PR with:
   - What you changed and why
   - How you tested it
   - Example output showing the improvement (if applicable)
