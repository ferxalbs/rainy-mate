# Repository Guidelines

## Project Structure & Module Organization

- `src/` is the React + TypeScript UI (entry points `src/main.tsx`, `src/App.tsx`).
- `src/components/`, `src/hooks/`, `src/services/`, `src/types/`, `src/assets/` hold UI, hooks, APIs, shared types, and assets.
- `src-tauri/` is the Rust backend + Tauri config (`src-tauri/src/`, `src-tauri/tauri.conf.json`).
- `public/` contains static assets; `docs/` and `plans/` hold documentation.
- Related subprojects: `rainy-api-v2/` (Bun API), `rainy-sdk/` (Rust SDK), `rainy-atm/` (service tooling).

## Build, Test, and Development Commands

- `pnpm install` installs root dependencies.
- Use `pnpm add <pkg>` and `pnpm remove <pkg>` for dependency changes (no npm).
- `pnpm run dev` starts the Vite dev server (UI only).
- `pnpm run tauri dev` runs the full desktop app.
- `pnpm run build` runs `tsc` then builds the Vite bundle.
- `pnpm run preview` serves the production bundle locally.
- `cd src-tauri && cargo test` runs Rust tests.
- `cd rainy-api-v2 && bun run dev` runs the API locally (Bun runtime).

## Coding Style, Architecture & UI Rules

- Rust does the work; TypeScript stays UI-only. Heavy logic belongs in `src-tauri/src/commands/` or `src-tauri/src/services/`.
- Keep modules small and single-purpose; avoid circular deps and oversized files.
- **Mandatory modularization rule**: skill/tool runtime code must be split by domain and responsibility, not grown in a single file.
  - Skill executor entrypoint stays thin in `src-tauri/src/services/skill_executor.rs` (routing, policy, shared guards only).
  - Tool arguments/schemas live in `src-tauri/src/services/skill_executor/args.rs`.
  - Tool catalog/registration lives in `src-tauri/src/services/skill_executor/registry.rs`.
  - Execution logic is separated per domain module (`filesystem.rs`, `shell.rs`, `web.rs`, `browser.rs`).
  - New tools must be added without turning the orchestrator into a monolith.
- No dead code. If future work must stay, mark it with `@deprecated`, `@TODO`, or `@RESERVED`.
- TS/TSX formatting matches existing files: 2-space indent, double quotes, trailing commas.
- Components are `PascalCase.tsx`; hooks are `useX.ts` in `src/hooks/`.
- For overlays/modals/cards, use `backdrop-blur-md` with a translucent background.

## Models & Dependencies

- Model catalogs live in `src/components/ai/UnifiedModelSelector.tsx` and `rainy-api-v2/src/utils/agentModelsConfig.ts` (keep both in sync).
- Add dependencies via the installer command (do not edit versions by hand):
- `pnpm add <package>`
- `pnpm remove <package>`
- Keep installs to the latest stable version unless a pin is required.
- Do not list or hardcode version numbers in this guide.

## Testing Guidelines

- Rust tests live in `src-tauri/src/*_tests.rs` and `src-tauri/src/test/`.
- Frontend tests are not wired in `package.json`; document any new test runner you add.

## Commit & Pull Request Guidelines

- Use Conventional Commits with scopes (e.g., `feat(agent-chat): add plan confirmation card`).
- PRs include summary, rationale, tests run, and linked issues; include screenshots for UI changes.
- For major changes, update `CHANGELOG.md` and bump versions in `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.

## Security & Configuration

- Keep secrets out of Git. Use local env files and OS keychains.
- Tauri permissions and window policy live in `src-tauri/tauri.conf.json`.

## Agent Capabilities & Skills

The agent connects to the Cloud Cortex via `rainy-atm` and executes local skills defined in `src-tauri/src/services/skill_executor.rs`.

### Available Skills

- **Filesystem**: `read_file`, `read_many_files`, `list_files`, `search_files`, `file_exists`, `get_file_info`, `read_file_chunk`, `write_file`, `append_file`, `mkdir`, `move_file`, `delete_file`
- **Browser**: `browse_url`, `open_new_tab`, `click_element`, `wait_for_selector`, `type_text`, `submit_form`, `go_back`, `screenshot`, `get_page_content`, `get_page_snapshot`, `extract_links`
- **Web**: `web_search`, `read_web_page`, `http_get_json`, `http_post_json`
- **Shell**: `execute_command`, `git_status`, `git_diff`, `git_log` (Allowed command family: `npm`, `cargo`, `git`, `ls`, `grep`, `echo`, `cat`)

### Adding New Skills

1. Define arguments/schema in `src-tauri/src/services/skill_executor/args.rs`.
2. Register the tool in `src-tauri/src/services/skill_executor/registry.rs`.
3. Implement handler logic in the matching domain module (`filesystem.rs`, `shell.rs`, `web.rs`, `browser.rs`).
4. Wire method dispatch in the same domain module (do not grow the orchestrator).
5. Update `src/constants/defaultNeuralSkills.ts` so the node exposes the method to Cloud Cortex.
6. Update `src/constants/toolPolicy.ts` with correct airlock level mapping.
7. Validate with `cd src-tauri && cargo check` and `pnpm exec tsc --noEmit`.

## Airlock Security

- **Level 0 (Safe)**: Auto-approved (Read-only).
- **Level 1 (Sensitive)**: Requires notification (Write). Can be auto-approved in Headless Mode.
- **Level 2 (Dangerous)**: Requires explicit approval (Execute/Delete).

## Troubleshooting

- **Agent not responding**: Check `src-tauri` terminal logs. Ensure `rainy-atm` is reachable.
- **Command rejected**: Check Airlock UI in `NeuralPanel`.
- **API Errors**: Verify `rainy-atm` credentials in `NeuralPanel`.
