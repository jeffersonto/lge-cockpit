import type { LgePhaseId } from "../types";

export const DEMO_TASK = {
  title: "ITM-2847: Migrar autenticação para OAuth2 com PKCE",
  description:
    "O sistema atual utiliza JWT simples com segredo compartilhado. Precisamos migrar para OAuth2 com PKCE (Proof Key for Code Exchange) para atender requisitos de compliance LGPD e reduzir superfície de ataque. Todos os clientes mobile e web precisam ser atualizados de forma gradual sem downtime.",
};

export const DEMO_ARTIFACTS: Record<LgePhaseId, string> = {
  planning: `# Plano de Implementação — ITM-2847

## Objetivo
Migrar o sistema de autenticação de JWT simples para OAuth2 com PKCE, atendendo requisitos de segurança e compliance LGPD.

## Análise do Estado Atual
- Autenticação: JWT stateless com segredo compartilhado (\`JWT_SECRET\`)
- Sessões: sem refresh token; token expira em 24h
- Endpoints afetados: \`/api/auth/login\`, \`/api/auth/logout\`, \`/api/users/me\`

## Arquitetura Proposta

### 1. Novos Endpoints OAuth2
- \`POST /oauth2/authorize\` — emite authorization code com code_challenge PKCE
- \`POST /oauth2/token\` — troca code por access_token + refresh_token
- \`POST /oauth2/revoke\` — revoga refresh token

### 2. Schema do Banco de Dados

\`\`\`sql
CREATE TABLE oauth_tokens (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  refresh_token TEXT NOT NULL UNIQUE,
  code_challenge TEXT,
  expires_at TIMESTAMPTZ NOT NULL,
  revoked_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ DEFAULT NOW()
);
CREATE INDEX idx_oauth_tokens_user ON oauth_tokens(user_id);
CREATE INDEX idx_oauth_tokens_refresh ON oauth_tokens(refresh_token)
  WHERE revoked_at IS NULL;
\`\`\`

### 3. Fluxo PKCE
1. Cliente gera \`code_verifier\` (64 bytes aleatórios, base64url)
2. Cliente calcula \`code_challenge = SHA256(code_verifier)\` → base64url
3. POST \`/oauth2/authorize\` com \`code_challenge\` → recebe \`code\` (TTL: 5min)
4. POST \`/oauth2/token\` com \`code\` + \`code_verifier\` → recebe tokens

### 4. Middleware de Validação
Substituir \`verifyJwt\` por \`verifyOAuthToken\` em todas as rotas protegidas.

## Estimativa de Tarefas

| Tarefa | Complexidade |
|--------|-------------|
| Schema migration | Baixa |
| OAuth2 endpoints | Alta |
| PKCE utilities | Média |
| Atualização do middleware | Média |
| Testes de integração | Alta |

## Riscos
- Sessões ativas podem ser invalidadas durante a migração — usar flag \`OAUTH_MIGRATION_MODE=compatible\`
- Clientes mobile precisam de atualização simultânea

## Critérios de Aceite
- [ ] Zero regressões em endpoints existentes
- [ ] Cobertura de testes ≥ 80%
- [ ] Documentação OpenAPI atualizada
- [ ] Aprovação da equipe de segurança
`,

  builder: `# Builder — ITM-2847

## Quick Context
All 6 tasks completed successfully — 6 files created, 4 modified.

| # | Task | Status | Notes |
|---|------|--------|-------|
| 1 | Schema migration | ✅ Done | \`migrations/002_oauth_tokens.sql\` created |
| 2 | OAuth2 endpoints | ✅ Done | authorize, token, revoke controllers |
| 3 | PKCE utilities | ✅ Done | SHA-256 + base64url in \`pkce.util.ts\` |
| 4 | Middleware update | ✅ Done | verifyJwt → verifyOAuthToken |
| 5 | Module registration | ✅ Done | OAuthModule registered in app |
| 6 | Integration tests | ✅ Done | Full PKCE flow covered |

---

## Files Changed
- **Path:** \`migrations/002_oauth_tokens.sql\` — **created** — OAuth tokens table with indexes
- **Path:** \`src/auth/oauth2/authorize.controller.ts\` — **created** — Authorization endpoint with PKCE code_challenge
- **Path:** \`src/auth/oauth2/token.controller.ts\` — **created** — Token exchange with PKCE verification
- **Path:** \`src/auth/oauth2/revoke.controller.ts\` — **created** — Token revocation (soft delete via revoked_at)
- **Path:** \`src/auth/oauth2/pkce.util.ts\` — **created** — Code verifier/challenge generation and verification
- **Path:** \`src/auth/middleware/verifyOAuthToken.ts\` — **created** — New auth middleware for OAuth2 tokens
- **Path:** \`src/auth/auth.module.ts\` — **modified** — Registered new OAuth2 controllers
- **Path:** \`src/auth/middleware/auth.middleware.ts\` — **modified** — Switched from verifyJwt to verifyOAuthToken
- **Path:** \`src/users/users.controller.ts\` — **modified** — Compatible with new JWT payload
- **Path:** \`src/app.module.ts\` — **modified** — Registered OAuthModule

## Technical Decisions
- **PKCE over plain OAuth2:** Required for public clients (mobile/SPA) per RFC 7636
- **Soft delete for revocation:** \`revoked_at\` timestamp instead of hard delete preserves audit trail
- **Partial index:** \`WHERE revoked_at IS NULL\` on refresh token index for query performance

## Verification Results
- Tests: 312/312 passing
- Build: OK
- Linter: 0 warnings

## Context for Reviewer
- Token exchange endpoint is the most complex — verify PKCE flow carefully
- Migration is backward-compatible with \`OAUTH_MIGRATION_MODE=compatible\`
- No rate limiting yet on \`/oauth2/token\` — flagged for review
`,

  review: `# Review — ITM-2847

## Quick Context
3 issues found, all fixed — ready for Guardian.

| # | Severity | Dimension | File | Issue | Fix Applied |
|---|----------|-----------|------|-------|-------------|
| 1 | Medium | Security | token.controller.ts | No rate limiting on /oauth2/token | Added ThrottlerGuard (10 req/min) |
| 2 | Low | Technical | token.controller.ts | PKCE failures not logged with userId/IP | Added structured logging |
| 3 | Low | Technical | authorize.controller.ts | Test coverage 72% (min: 80%) | Added edge case tests → 82% |

| Dimension | Issues Found | Issues Fixed | Residual Risk |
|---|---|---|---|
| Plan Adherence | 0 | 0 | low |
| Technical Quality | 2 | 2 | low |
| Security & Business | 1 | 1 | low |

---

## Files Modified by Reviewer
- **Path:** \`src/auth/oauth2/token.controller.ts\`
  - **What was wrong:** Missing rate limiting and insufficient PKCE failure logging
  - **What was fixed:** Added \`@UseGuards(ThrottlerGuard)\` + structured error logging with userId and IP
- **Path:** \`src/auth/oauth2/authorize.controller.spec.ts\`
  - **What was wrong:** Coverage at 72%, below 80% threshold
  - **What was fixed:** Added tests for expired codes, malformed challenges, concurrent requests

## Verification Results After Fixes
- Tests: 312/312 passing
- Build: OK
- Coverage: authorize.controller 82%, token.controller 87%, pkce.util 100%

## Notes for Guardian
- \`X-Request-Id\` header propagation is not implemented — low priority, not in plan scope
- Rate limiting uses in-memory store (ThrottlerModule default) — adequate for single instance
`,

  guardian: `# Guardian — ITM-2847

## Quick Context
Final verdict: **APPROVED**

| Criterion | Status | Evidence | Notes |
|---|---|---|---|
| Plan Adherence | ✅ | All 6 tasks implemented | — |
| Module Consistency | ✅ | Interfaces consistent across OAuth2 module | — |
| Functional Completeness | ✅ | All business requirements met | — |
| Integration Integrity | ✅ | End-to-end PKCE flow verified | — |
| Error Handling | ✅ | All error paths covered | — |
| Test Coverage | ✅ | 82-100% per controller | — |

| Dimension | Builder | Reviewer | Guardian |
|---|---|---|---|
| Issues Found | 0 | 3 | 0 |
| Issues Fixed | 0 | 3 | 0 |
| Tests Passing | ✅ | ✅ | ✅ |
| Build | ✅ | ✅ | ✅ |

---

## Residual Risk Resolution
- **Flagged:** \`X-Request-Id\` header propagation missing
  - **Resolution:** Acceptable — not in plan scope, low priority. Tracked for next sprint.
- **Flagged:** Rate limiting uses in-memory store
  - **Resolution:** Acceptable for single-instance deployment. Redis-backed throttling recommended for horizontal scaling.

## Verification Results
- Tests: 312/312 passing
- Build: OK
- Coverage: authorize 82%, token 87%, pkce 100%
- Impact: 6 files created, 4 modified (+487/-12 lines)

## Final Verdict
The OAuth2 with PKCE implementation is technically sound and production-ready. All acceptance criteria met: zero regressions, coverage ≥ 80%, OpenAPI updated, security review passed. Recommended deployment strategy: canary at 10% traffic, monitor for 24h, then disable \`LEGACY_AUTH_ENABLED\`.
`,
};
