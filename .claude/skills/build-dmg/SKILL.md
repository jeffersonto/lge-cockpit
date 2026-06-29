---
name: build-dmg
description: Build the LGE Cockpit production macOS bundle and stage the resulting .dmg in the project root. Use whenever the user asks to "gerar dmg", "buildar dmg", "build dmg", "criar release", "empacotar app", "create release bundle", or otherwise wants a distributable macOS installer at the repo root. The skill runs `pnpm tauri build`, locates the generated bundle under `src-tauri/target/release/bundle/dmg/`, and copies it to `<repo-root>/LGE Cockpit_<version>_<arch>.dmg`.
---

# Build DMG — LGE Cockpit

Produz o instalador `.dmg` de produção do LGE Cockpit e o coloca na raiz do repositório, com o nome canônico que o Tauri gera.

## Pré-condições

- Rodando em macOS (o bundle `.dmg` só é gerado neste sistema operacional)
- `pnpm` e `cargo` disponíveis no PATH (mesmas dependências de `pnpm tauri dev`)
- Diretório de trabalho na raiz do projeto (`/Users/jefcosta/workspace/lge-cockpit` ou equivalente)
- Versão sincronizada nos 3 arquivos: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml` (ver `.claude/rules/version-sync.md`)

## Passos

### 1. Rodar o build de produção

```bash
pnpm tauri build
```

Este comando executa:
1. `pnpm build` (`tsc && vite build`) — compila o frontend para `dist/`
2. `cargo build --release` — compila o binário Rust em modo release
3. Empacota o `.app` em `src-tauri/target/release/bundle/macos/LGE Cockpit.app`
4. Empacota o `.dmg` em `src-tauri/target/release/bundle/dmg/LGE Cockpit_<VERSION>_<ARCH>.dmg`

O log final lista o caminho exato do `.dmg` gerado — use-o no próximo passo. Tempo típico: 30–60 segundos em máquina já aquecida (Rust incremental); 3–5 minutos em build limpo.

### 2. Copiar o `.dmg` para a raiz

Pegue a versão de `package.json` e a arquitetura do host, e copie o arquivo. Não renomeie — preserve o nome canônico do Tauri:

```bash
VERSION=$(node -p "require('./package.json').version")
ARCH=$(uname -m)   # arm64 → aarch64, x86_64 → x64
case "$ARCH" in
  arm64)  ARCH=aarch64 ;;
  x86_64) ARCH=x64 ;;
esac
DMG="LGE Cockpit_${VERSION}_${ARCH}.dmg"
cp "src-tauri/target/release/bundle/dmg/${DMG}" "./${DMG}"
ls -lh "./${DMG}"
```

### 3. Confirmar com o usuário

Reporte o caminho absoluto, tamanho e versão. Exemplo:

> DMG gerado em `/Users/.../lge-cockpit/LGE Cockpit_0.6.0_aarch64.dmg` (5.7 MB).

## Importante

- **Não comite o `.dmg`.** O padrão `*.dmg` já está em `.gitignore`. O CLAUDE.md alerta sobre artefatos `.dmg` esquecidos na raiz — se o usuário pedir um commit logo depois, confirme que o `.dmg` não está no staging.
- **Não use `git add -A` / `git add .`** após gerar o `.dmg`. Adicione arquivos por nome.
- **Versão errada no nome do arquivo** indica desalinhamento entre `package.json`, `tauri.conf.json` e `Cargo.toml`. Pare e siga `.claude/rules/version-sync.md` antes de continuar.
- **Arquitetura cruzada** (build `x64` numa máquina `arm64` ou vice-versa) requer flags adicionais no `tauri build` (`--target`). Esta skill cobre apenas o build nativo do host.

## Quando NÃO usar esta skill

- O usuário só quer rodar a aplicação localmente → use `pnpm tauri dev`.
- O usuário só quer compilar para validar → use `cd src-tauri && cargo build` (debug, sem bundle) ou `pnpm build` (frontend apenas).
- O usuário quer publicar uma release no GitHub → o build é só o primeiro passo; a publicação envolve tag, release notes e upload, e está fora do escopo desta skill.
