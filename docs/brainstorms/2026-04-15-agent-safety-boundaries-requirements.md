---
date: 2026-04-15
topic: agent-safety-boundaries
---

# Agent Safety Boundaries

## Problem Frame

Current agent profiles define model, prompt, MCP services, and tools, but they do not define what local project or directory an agent is allowed to operate on. As the product moves from chat-only behavior toward file access, project scanning, and command execution, that missing boundary creates two problems:

- users cannot predict what an agent is allowed to touch
- the system has no explicit line between acceptable workspace access and unsafe overreach

This matters even before full sandboxing exists. Without a declared working scope, the product risks feeling unsafe, and later safety controls become harder to layer in consistently.

## Requirements

**Boundary Model**
- R1. Each agent profile must support a `workDirectory` concept that defines the local workspace the agent is intended to operate within.
- R2. Each agent conversation must snapshot the effective working directory at conversation creation, just like other runtime-defining profile settings.
- R3. The product must treat the working directory as a runtime boundary for local workspace operations, not only as prompt guidance.

**Capability Gating**
- R4. Agents that can read files, scan projects, browse local directories, or execute local commands must not be allowed to operate outside their effective working directory.
- R5. Agents without a configured working directory must be restricted to chat-safe capabilities and must not be presented as able to perform local project actions.
- R6. Destructive or high-risk actions must remain separately gated even when they occur inside the allowed working directory.

**User Understanding**
- R7. Agent management UI must explain what the working directory means in product terms: what it enables, what it restricts, and what it does not guarantee.
- R8. Workspace and conversation UI must surface enough boundary context that the user can tell which project or directory the current agent session is operating against.

**Security Positioning**
- R9. The first safety milestone must prioritize hard path-boundary enforcement for local operations before introducing more advanced sandboxing language.
- R10. The product must not imply that `workDirectory` alone is equivalent to full sandbox, container isolation, or complete system-level security.

## Success Criteria

- Users can tell which local workspace an agent is scoped to before they ask it to act.
- Local file and project operations outside the allowed boundary are rejected by runtime checks rather than relying only on instructions.
- Agents that lack a working directory do not appear to have unsafe local access capabilities.
- The system has a clear conceptual base for later permission tiers and stronger isolation.

## Scope Boundaries

- This phase does not require full containerization, OS sandboxing, or per-agent virtual machines.
- This phase does not require designing every future permission tier in detail.
- This phase does not require solving multi-root workspaces or advanced project switching.
- This phase does not require claiming security guarantees beyond the controls that are actually enforced.

## Key Decisions

- `workDirectory` should exist because agent capability without explicit local scope is both a product trust problem and a future security problem.
- `workDirectory` should be treated as a hard boundary for local workspace operations, not just a convenience default.
- `workDirectory` alone should not be marketed or understood as full sandboxing.
- In the first phase, path-boundary enforcement and capability gating are more important than deeper infrastructure isolation.
- Agents without local workspace scope should remain usable for plain chat, but not for local project actions.

## Dependencies / Assumptions

- Local file access, project scanning, and command execution will continue to expand over time.
- The current agent runtime already snapshots conversation-defining settings, so working directory should follow the same product rule.
- Some MCP tools may eventually need classification between chat-safe tools and local-workspace tools.

## Outstanding Questions

### Resolve Before Planning
- None.

### Deferred to Planning
- [Affects R4][Technical] Which current and planned tools must be classified as local-workspace operations that require hard boundary checks?
- [Affects R7][Technical] Should the first-phase field store a raw filesystem path, or should it immediately bind to a higher-level project/workspace entity?
- [Affects R6][Needs research] Which actions should count as high-risk and require confirmation even inside the allowed directory?

## Next Steps

-> /ce:plan for structured implementation planning
