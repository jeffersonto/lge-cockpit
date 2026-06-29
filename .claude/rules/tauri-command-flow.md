---
globs: ["src-tauri/src/commands/*.rs", "src-tauri/src/lib.rs", "src/lib/tauri.ts", "src/stores/*.ts"]
---

# Tauri command wiring

Every IPC command crosses 4 layers. Skipping any layer compiles cleanly but fails silently at runtime. When adding or renaming a command, edit all four files in the same change.

## Required steps

1. **Define** the function in `src-tauri/src/commands/<area>.rs` with `#[tauri::command]`. Return `Result<T, String>` so JS can `.catch()`.
2. **Register** in `src-tauri/src/lib.rs` inside `tauri::generate_handler![ ... ]` (see `lib.rs:54`). Keep entries grouped by area to match the file split.
3. **Type** the wrapper in `src/lib/tauri.ts` — `invoke("snake_case_name", { camelCaseArgs })`. The arg keys MUST be camelCase even though the Rust signature uses snake_case (Tauri converts automatically only when serde renames are absent).
4. **Call** from a Zustand store in `src/stores/*.ts`, never directly from a component.

## Correct

```rust
// src-tauri/src/commands/tasks.rs
#[tauri::command]
pub async fn delete_task(state: State<'_, AppState>, id: String) -> Result<(), String> { ... }
```

```rust
// src-tauri/src/lib.rs
.invoke_handler(tauri::generate_handler![
    commands::tasks::delete_task,
    // ...
])
```

```ts
// src/lib/tauri.ts
export async function deleteTask(id: string): Promise<void> {
  return invoke("delete_task", { id });
}
```

```ts
// src/stores/taskStore.ts
deleteTask: async (id) => { await api.deleteTask(id); /* update state */ }
```

## Incorrect — silent failures

- Defining the command but forgetting `generate_handler![]` → JS gets `Command not found`.
- Registering in `lib.rs` but skipping `src/lib/tauri.ts` → components must use raw `invoke()` strings, bypassing types.
- Calling `invoke()` directly from a component → bypasses store, breaks state consistency.
- Forgetting to add the new command file to `src-tauri/src/commands/mod.rs` → won't compile, but easy to fix.

## When renaming

A rename touches all 4 files plus any i18n keys that mention the action. Search `grep -rn "old_name" src-tauri/src/ src/` before claiming the rename is done.
