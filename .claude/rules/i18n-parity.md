---
globs: ["src/i18n/*.json", "src/**/*.tsx", "src/**/*.ts"]
---

# i18n key parity

The app ships three locales: `pt-BR` (default), `en`, `es`. Translation files are at `src/i18n/{pt-BR,en,es}.json`. Each currently has the same set of keys (~274). When a key is added to one file but not the others, missing locales render the literal key string in the UI.

## Required edits when introducing UI text

Every new `t("namespace.key")` call requires the same `namespace.key` path in all three JSON files. Pick the namespace by feature area: `tasks`, `lge`, `settings`, `git`, `attachments`, etc.

```json
// src/i18n/pt-BR.json
{ "tasks": { "create": "Criar tarefa" } }

// src/i18n/en.json
{ "tasks": { "create": "Create task" } }

// src/i18n/es.json
{ "tasks": { "create": "Crear tarea" } }
```

## Verification

After editing any `i18n/*.json` file, confirm the key counts still match:

```bash
node -e 'const c=p=>Object.keys(require("./"+p).en?{}:{...require("./"+p)}).length;
  const flat=(o,p="")=>Object.entries(o).flatMap(([k,v])=>typeof v==="object"?flat(v,p+k+"."):[p+k]);
  for (const f of ["pt-BR","en","es"]) console.log(f, flat(require("./src/i18n/"+f+".json")).length);'
```

Or simpler — sort and diff:

```bash
for f in pt-BR en es; do
  jq -r 'paths(scalars) | join(".")' src/i18n/$f.json | sort > /tmp/$f.keys
done
diff /tmp/pt-BR.keys /tmp/en.keys
diff /tmp/en.keys /tmp/es.keys
```

The diffs must be empty.

## Don't

- Hard-code user-visible strings in components (`<button>Save</button>`). Use `t("...")`.
- Add a key only to `pt-BR.json` "to fix later". The fallback chain renders the key path verbatim, not the pt-BR value.
- Rename a key in one file. Rename in all three.
- Use string interpolation in keys (`t("tasks." + name)`). Static keys only — easier to grep and verify parity.
