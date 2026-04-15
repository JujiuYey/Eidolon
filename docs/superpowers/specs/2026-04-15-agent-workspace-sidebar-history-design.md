# Agent Workspace Sidebar History Design

## Summary

Align the agent workspace with mainstream chat products:

- `workspace` becomes a single chat panel
- recent conversations move into the global layout sidebar
- selecting a sidebar conversation reopens it in the workspace

## Product Decisions

### Workspace Responsibilities

- render only the active conversation
- never render agent selection inside the page
- never render a conversation list inside the page

### Sidebar Responsibilities

- keep the existing app navigation
- add a `最近会话` section below navigation
- show recent conversations across all agents
- allow continuing an existing conversation from the sidebar
- allow creating a new conversation from the current conversation's agent context

### Routing

- `/agent/workspace?conversation=<id>` opens an existing conversation
- `/agent/workspace?agent=<id>` creates a new conversation for that agent, then rewrites the route to `conversation=<id>`
- legacy `/agent/:id` redirects into `/agent/workspace?agent=<id>`

## Data Requirements

- backend must provide a recent-conversations query across all agents
- frontend sidebar and workspace must share one source of truth for active conversation state

## Out of Scope

- changing agent assignment for an existing conversation
- redesigning the `/agent` management page
- removing unrelated demo pages
