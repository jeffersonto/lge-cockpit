---
globs: ["src/stores/*.ts"]
---

# Zustand store conventions

All cross-cutting state lives in `src/stores/*.ts`. Components call store actions, never `invoke()` directly. Async actions wrap a single `api.*` call from `src/lib/tauri.ts`.

## Error handling — pick one pattern per action

The current code mixes three patterns. New actions should pick one explicitly based on whether the caller needs to react to the error.

### Pattern A — re-throw (action's caller decides)

Use when a component awaits the action and shows feedback (toast, error banner, dialog) on failure.

```ts
createTask: async (repositoryId, title, description) => {
  try {
    const task = await api.createTask({ repository_id: repositoryId, title, description });
    set((state) => ({ tasks: [...state.tasks, task] }));
  } catch (error) {
    console.error("Failed to create task:", error);
    throw error;  // caller awaits and reacts
  }
},
```

### Pattern B — swallow with logging (background fetch)

Acceptable for fire-and-forget refreshes where the UI degrades gracefully (empty list, stale data). Always log; never silently `catch {}`.

```ts
fetchTasks: async (repositoryId) => {
  set({ loading: true });
  try {
    const tasks = await api.listTasks(repositoryId);
    set({ tasks, loading: false });
  } catch (error) {
    console.error("Failed to fetch tasks:", error);
    set({ loading: false });  // always reset loading
  }
},
```

### Pattern C — no try/catch (let it bubble)

Fine when the action is short and the caller is expected to wrap it. Don't mix this with `set` calls that leave inconsistent state on partial failure.

## Required practices

- Always reset `loading: false` in both success and error branches.
- Update local state only after the backend call resolves — never optimistically without rollback.
- Keep cross-store calls explicit (`useRepositoryStore.getState().fetchRepositories()`) rather than circular imports.
- Prefer `set((state) => ({ ... }))` (functional form) when the new value depends on the previous state, especially for arrays and maps.

## Don't

- Don't `console.error` and also `throw` — the caller's catch already logs. Pick one.
- Don't put business logic (validation, formatting) in the store. Stores transport state; logic belongs in `lib/` or backend.
- Don't store derived data — compute it in selectors or components.
