# ADR 001: Direct Jira API Integration Over MCP

ADR Number: 001
Title: Direct Jira API Integration Over MCP
Date: 2026-07-01
Responsible: LGE Cockpit Team
Status: Accepted

## Context

Jira integration in LGE Cockpit was originally implemented through the Model Context Protocol (MCP), specifically by invoking the Claude Code CLI as a subprocess with the Atlassian MCP configured. This approach worked for the MVP, but introduced hard-to-control external dependencies: every machine running the app needed Claude CLI installed, an Atlassian MCP configured and authenticated (via OAuth or marketplace plugin), and a working Node/NPX environment for the remote fallback.

As the product matured, it became necessary to reduce installation friction and ensure that Jira issue import worked predictably for all users, regardless of how their development machines were configured.

## Decision

We decided to completely replace the MCP-based integration with direct calls to the Jira Cloud REST API, using API token authentication (email + token). MCP, Claude CLI, and all MCP detection/configuration logic will be removed from the import flow.

The new integration will be encapsulated in its own domain module — the `JiraClient` — with a port/adapter structure that isolates HTTP (`reqwest`) and format conversion from the rest of the application.

## Justification

The decision to integrate directly with the Jira API was based on the following reasons:

- **Lower deployment friction**: With the direct API, the user only needs to provide email, API token, and Jira base URL in the app settings. There is no longer a need to install Claude CLI, configure MCPs, or handle third-party OAuth flows.
- **Predictability**: Import behavior now depends only on the official Atlassian API, eliminating variations caused by Claude CLI versions, MCP tool naming conventions (`getJiraIssue` vs `jira_get_issue`), and marketplace authentication states.
- **Security and governance**: Credentials (email + token) are stored in the app's SQLite database, following the existing secrets pattern. Tokens are no longer passed through CLI prompts or subprocess environment variables.
- **Foundation for sync-back**: A generic `JiraClient` allows future operations — such as posting comments and transitioning status — to reuse the same authentication and HTTP infrastructure, avoiding a painful refactor later.
- **Reliable description conversion**: The issue ADF description is obtained through the API's `renderedFields` (ready-to-use HTML) and converted to Markdown. This reuses Atlassian's own renderer, preserving the required fidelity without relying on LLM prompts.

## Alternatives Considered

The following alternatives were considered:

- **Keep MCP as the primary path**: This would retain the flexibility of delegating integration to a generic protocol, but would perpetuate the dependency on Claude CLI and per-machine configuration — the exact problem we want to solve.
- **Keep MCP as an optional fallback**: This seemed like a safe middle ground, but it would double the test surface and keep the coupling that this decision aims to remove. Fallbacks make sense during migration, not as a final architecture.
- **Use Jira OAuth 2.0 (3LO)**: More secure in theory, but requires a browser authorization flow and callback handling on the desktop, increasing complexity without proportional gain for a personal/small-team app. API tokens are the pattern recommended by Atlassian for scripts and desktop applications.
- **Write a custom ADF → Markdown converter in Rust**: This would give more control, but would reimplement part of Atlassian's renderer. We chose to use `renderedFields` (HTML) + an HTML-to-Markdown converter, which is simpler and equally faithful.

## Consequences

The choice to integrate directly with the Jira API brings the following consequences:

- **Legacy code removal**: All MCP code (`detect_atlassian_mcp`, `resolve_mcp_config`, `resolve_npx_path`, `run_jira_diagnostic`, MCP prompts) will be removed. This simplifies the codebase, but requires the `import_jira_task` command to be rewritten and `run_jira_diagnostic` to be replaced by a connection test.
- **New Rust dependencies**: `reqwest` (HTTP client) and an HTML-to-Markdown crate (e.g., `html2md`) will be added.
- **New configuration surface**: The Settings screen will gain fields for Jira email and API token, plus a "Test connection" button.
- **Previously imported tasks remain valid**: Tasks with `source = 'jira'` and a populated `jira_key` do not require migration; only the future import mechanism changes.
- **Controlled initial scope**: The `JiraClient` will be modeled as a generic port/adapter from the start, but the first delivery will implement only issue reading (`GET /rest/api/3/issue/{key}`), leaving comments and transitions for future iterations.
- **Internal error model**: API errors will be modeled as a structured domain enum (`JiraError`) and converted to user-friendly messages only at the Tauri command boundary.
