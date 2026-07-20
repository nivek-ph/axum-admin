# React Admin Console

`apps/desktop` is the maintained React Admin Console. It currently runs as a browser SPA; a future
Tauri shell remains a separate enhancement. The previous Vue + Tauri implementation is preserved in
the repository's `v1.1.0` tag.

```bash
pnpm install
pnpm dev
```

The API base defaults to `http://127.0.0.1:3000/api` and can be overridden with
`VITE_API_BASE_URL`.

## Verification

```bash
pnpm lint
pnpm format:check
pnpm test
pnpm typecheck
pnpm build
```

The application uses browser-history routes. A production static host must return `index.html`
for unknown non-API paths so direct navigation and reloads work. The Vite development and preview
servers already provide this SPA fallback.
