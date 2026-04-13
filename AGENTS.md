# AGENTS.md

## Purpose

This repository must be handled by OpenAI Codex as a software engineering and infrastructure project, with strong emphasis on:

- application development
- infrastructure as code
- Docker Compose orchestration
- maintainability
- safe, reversible changes
- precise documentation of every relevant modification

The agent must behave like a disciplined senior engineer working in an existing production-capable repository.

---

## Language and communication rules

- Always communicate with the user in **pt-BR**.
- Write code, commit suggestions, filenames, identifiers, comments, and technical documentation in the language that best fits the repository conventions. Prefer **English for code and technical artifacts** unless the repository clearly uses another standard.
- Be direct, technical, and objective.
- Do not make up APIs, flags, configuration keys, framework behaviors, or library syntax.
- When uncertain, validate before changing.

---

## Mandatory MCP usage

The agent must use the MCP tools below as part of its default workflow.

### 1) Context7 — mandatory for language/framework/library standards

Use **Context7** whenever writing, refactoring, or reviewing code, configs, or infrastructure definitions that depend on a language, framework, library, tool, or platform convention.

This includes, but is not limited to:

- Python
- TypeScript / JavaScript
- Bash
- Docker / Docker Compose
- Traefik
- Django / FastAPI / Flask
- Node.js ecosystems
- database clients and ORMs
- CI/CD tooling
- infrastructure-related libraries and CLIs

#### Rules for Context7 usage

- Before generating or changing code, fetch the **current official or version-appropriate documentation** through Context7.
- Use Context7 to confirm:
  - syntax
  - idioms
  - naming conventions
  - deprecations
  - recommended patterns
  - breaking changes
  - official examples
- Do not rely on memory when the code depends on framework/library behavior.
- When editing existing code, align the implementation with:
  1. repository conventions
  2. language idioms
  3. official documentation retrieved through Context7

#### Output quality rules

Any code produced must follow:

- the standard style of the target language
- the dominant conventions already present in the repository
- simplicity over cleverness
- explicitness over magic
- small focused functions/modules
- predictable error handling
- clear names
- minimal hidden side effects
- low-coupling changes

#### Docker Compose specific rules

When editing `compose.yaml`, `docker-compose.yml`, or related infra files:

- prefer clarity and explicitness over compact tricks
- preserve service names unless a rename is required
- avoid unnecessary breaking changes in:
  - networks
  - volumes
  - ports
  - labels
  - environment variables
  - healthchecks
  - restart policies
  - dependency relationships
- keep Compose definitions modular and readable
- prefer anchors/extensions only when they improve maintainability without obscuring behavior
- validate official Compose field semantics with Context7 before introducing or changing:
  - `depends_on`
  - `healthcheck`
  - `profiles`
  - `deploy`
  - `configs`
  - `secrets`
  - `networks`
  - `extends`
  - logging options
  - resource constraints
  - lifecycle-related behavior

---

### 2) Basic Memory — mandatory for documenting changes and corrections

Use **Basic Memory** as the canonical memory layer for documenting meaningful work done in the repository.

The agent must record relevant implementation knowledge during the session, especially:

- what was changed
- why it was changed
- what bug/problem was fixed
- what assumption was validated
- what tradeoff was chosen
- what follow-up work remains
- what constraints were discovered
- what risks or caveats exist

#### What must be documented in Basic Memory

Document at least the following whenever applicable:

- feature additions
- refactors
- bug fixes
- infrastructure changes
- config changes
- dependency updates
- behavioral changes
- migrations
- discovered repository conventions
- important debugging findings
- root cause analysis
- rejected alternatives worth remembering

#### Documentation format rules

Each memory entry should be concise, technical, and useful for future agents.

Prefer documenting with this structure:

- **Context**: what area of the system was touched
- **Change**: what was modified
- **Reason**: why
- **Impact**: expected behavior/result
- **Caveats**: risks, migration notes, or pending follow-up

Do not store noise.
Do not store trivial edits.
Store decisions and findings that matter later.

---

### 3) Serena — mandatory for code indexing and semantic navigation

Use **Serena** as the primary semantic code indexing and navigation layer.

The agent must use Serena to understand the codebase before making non-trivial changes.

#### Mandatory Serena workflow

Before editing code in an unfamiliar or non-trivial area:

1. activate the project in Serena
2. index or load the project context if needed
3. inspect relevant symbols, references, and structure
4. identify the real impact surface before editing
5. prefer semantic navigation over blind text search when possible

#### Serena must be used for

- discovering entry points
- locating symbol definitions
- finding references
- mapping module relationships
- understanding service boundaries
- tracing config usage
- identifying dead code candidates
- estimating blast radius before refactors

#### Editing rules with Serena

- Do not patch code blindly.
- Understand the symbol graph before changing shared code.
- When changing a public function, shared utility, config contract, or service interface, inspect usages first.
- Prefer minimal-impact edits that preserve existing contracts unless the task explicitly requires breaking changes.

---

## Standard execution workflow

For every substantial task, follow this order:

1. **Understand the task**
   - infer the real objective
   - inspect relevant files and architecture boundaries

2. **Use Serena first**
   - identify where the change belongs
   - trace references and dependencies
   - understand the current implementation

3. **Use Context7 before coding**
   - confirm current official patterns and syntax
   - verify framework/tool behavior
   - avoid deprecated or invented solutions

4. **Implement**
   - make the smallest correct change that fully solves the problem
   - keep the solution maintainable
   - avoid unrelated refactors unless necessary

5. **Validate**
   - run or propose appropriate validation steps
   - check for syntax, config, and logic consistency
   - inspect adjacent impact areas

6. **Document in Basic Memory**
   - record what changed, why, and any caveats
   - persist useful debugging and design knowledge

7. **Report clearly**
   - summarize what was changed
   - mention important side effects
   - mention validation performed
   - mention pending risks or follow-ups if any

---

## Coding rules

### General

- Prefer boring, correct, maintainable code. Fancy code is often a clown car in disguise.
- Match the repository’s architecture unless there is a clear reason to improve it.
- Keep functions and modules cohesive.
- Avoid duplication, but do not over-abstract prematurely.
- Preserve backward compatibility unless the task explicitly requires change.
- Never mix unrelated fixes in the same change without stating it clearly.

### Readability

- Use descriptive names.
- Avoid misleading abbreviations.
- Keep control flow straightforward.
- Prefer explicit data flow.
- Write comments only when they add real value.
- Do not comment obvious code.

### Error handling

- Fail loudly when silent failure would hide operational issues.
- Return useful errors.
- Preserve actionable logs.
- Do not swallow exceptions without reason.
- For infra/config changes, prefer observable failures over hidden misconfiguration.

### Security and secrets

- Never hardcode secrets.
- Prefer environment variables or proper secret mechanisms.
- Do not expose internal services unnecessarily.
- Apply least-privilege thinking to containers, networks, and credentials.

### Config and infra hygiene

- Keep environment variables documented and consistently named.
- Avoid magic ports, paths, and hostnames without explanation.
- Use healthchecks when operationally meaningful.
- Respect startup/readiness semantics.
- Keep reverse-proxy labels and routing rules explicit and consistent.

---

## Change scope rules

- Make focused changes.
- Do not rewrite large sections unless necessary.
- Do not introduce new dependencies without justification.
- Do not upgrade versions casually.
- Do not change formatting in unrelated files.
- Do not rename files/symbols unless it improves correctness or maintainability enough to justify the churn.

---

## Validation rules

Whenever possible, validate changes with the tools appropriate to the stack, such as:

- linters
- formatters
- type checkers
- unit tests
- integration tests
- container config validation
- compose config inspection
- service startup checks

For Docker Compose and infrastructure changes, explicitly verify where relevant:

- service naming
- network attachment
- volume mounts
- exposed ports
- reverse-proxy labels
- healthchecks
- dependency behavior
- environment variable completeness

If something cannot be validated directly, state that clearly.

---

## Reporting format

At the end of a task, provide a concise report containing:

- what changed
- why it changed
- files affected
- validation performed
- risks / caveats
- next recommended step, if applicable

---

## Non-negotiable rules

- Use **Serena** for semantic code understanding before non-trivial edits.
- Use **Context7** before writing or changing framework/library/tool-specific code.
- Use **Basic Memory** to document meaningful changes, corrections, and findings.
- Do not invent documentation.
- Do not guess syntax when official docs can be consulted.
- Do not apply broad refactors without necessity.
- Do not leave important decisions undocumented.

