# PRD: rustLink Codebase Refactoring and Reorganization

## Introduction

The rustLink codebase has grown organically, resulting in several files that have become too large and difficult to maintain. The `routes.rs` file in particular contains 510+ lines mixing route handlers, state management, router configuration, and utility functions. This PRD outlines a comprehensive refactoring to improve code organization while maintaining the existing layered architecture and avoiding any breaking changes to the API or database schema.

## Goals

- Break down large files (`routes.rs` at 510 lines, `config.rs` at 315 lines, `main.rs` at 317 lines) into smaller, focused modules
- Establish clear module boundaries with single responsibilities
- Improve discoverability - make it easy to find where to add new features
- Maintain existing functionality without breaking changes
- Keep the current layered architecture pattern (HTTP → Application → Infrastructure → Persistence)
- Reduce cognitive load when navigating the codebase

## User Stories

### US-001: Reorganize routes module into subdirectories
**Description:** As a developer, I want the routes module organized into logical subdirectories so that I can easily find and modify specific handlers.

**Acceptance Criteria:**
- [x] Create `src/routes/` directory with modular structure
- [x] Split `routes.rs` into focused files: `mod.rs`, `handlers.rs`, `health.rs`, `router.rs`, `types.rs`
- [x] All imports and module declarations resolve correctly
- [x] `cargo check` passes with no errors
- [x] `cargo test` passes all existing tests
- [x] `cargo clippy` produces no new warnings

**Status:** ✅ Completed - Commit b4731cf

### US-002: Extract route handlers into separate modules by domain
**Description:** As a developer, I want route handlers grouped by domain (url, auth, admin) so I can quickly locate the code for a specific endpoint.

**Acceptance Criteria:**
- [x] Create `src/routes/url_handlers.rs` for URL-related endpoints (create, resolve, info)
- [x] Create `src/routes/auth_handlers.rs` for authentication endpoints (login)
- [x] Create `src/routes/admin_handlers.rs` for admin-only endpoints (stats, list, delete)
- [x] Move health check logic to `src/routes/health.rs`
- [x] Extract shared helper functions to `src/routes/helpers.rs`
- [x] All handlers maintain existing function signatures and behavior
- [x] `cargo check` passes with no errors
- [x] `cargo test` passes all existing tests

**Status:** ✅ Completed - Commit bf19d46

### US-003: Extract router configuration into separate module
**Description:** As a developer, I want router setup (rate limiting, CORS, middleware) in a dedicated module so changes to routing don't require scrolling through handler code.

**Acceptance Criteria:**
- [x] Create `src/routes/router.rs` with `create_router()` function
- [x] Move rate limiting configuration to `router.rs`
- [x] Move CORS configuration to `router.rs`
- [x] Middleware stack configuration consolidated in `router.rs`
- [x] Router structure uses clear separation (sensitive, public, health route groups)
- [x] `cargo check` passes with no errors

**Status:** ✅ Completed - Commit b4731cf (accomplished as part of US-001)

### US-004: Extract route-specific types and responses
**Description:** As a developer, I want request/response types specific to routes in a separate module so the type definitions don't clutter the handler code.

**Acceptance Criteria:**
- [x] Create `src/routes/types.rs` for route-specific structs
- [x] Move `ListUrlsQuery`, `HealthCheckResponse`, `HealthStatus` to `types.rs`
- [x] Re-export commonly used types from `routes::mod.rs`
- [x] All existing code that uses these types continues to work
- [x] `cargo check` passes with no errors

**Status:** ✅ Completed - Commit b4731cf (accomplished as part of US-001)

### US-005: Move AppState to dedicated state module
**Description:** As a developer, I want application state management in a dedicated module so the shared state structure is clearly documented and easy to modify.

**Acceptance Criteria:**
- [x] Create `src/state.rs` module
- [x] Move `AppState` struct from `routes.rs` to `state.rs`
- [x] Add documentation comments to all `AppState` fields
- [x] Update all imports across the codebase
- [x] `cargo check` passes with no errors

**Status:** ✅ Completed - Commit 01d6fa1

### US-006: Refactor config.rs into focused submodules
**Description:** As a developer, I want configuration split into logical sections (server, database, cache, rate limiting) so I can quickly find and modify specific config areas.

**Acceptance Criteria:**
- [x] Create `src/config/` directory
- [x] Split into: `mod.rs`, `server.rs`, `database.rs`, `cache.rs`, `rate_limit.rs`, `url.rs`, `auth.rs`
- [x] Keep `Config` struct as the unified entry point in `mod.rs`
- [x] Each submodule handles validation for its specific section
- [x] Maintain backward compatibility with existing environment variable names
- [x] `cargo check` passes with no errors
- [x] `cargo test` passes all config validation tests

**Status:** ✅ Completed - Commit 068da9d

### US-007: Extract server startup logic from main.rs
**Description:** As a developer, I want server startup logic in a separate module so `main.rs` focuses only on CLI parsing and application entry.

**Acceptance Criteria:**
- [ ] Create `src/server.rs` module
- [ ] Move server startup logic to `server.rs`
- [ ] Move graceful shutdown handling to `server.rs`
- [ ] Move worker spawning logic to `server.rs`
- [ ] `main.rs` contains only CLI parsing and minimal orchestration
- [ ] `cargo check` passes with no errors
- [ ] Server starts and stops correctly with `cargo run -- server`

### US-008: Extract short code generation to dedicated service
**Description:** As a developer, I want short code generation logic in a dedicated module so it's reusable and testable independently from HTTP handlers.

**Acceptance Criteria:**
- [x] Create `src/services/` directory
- [x] Create `src/services/mod.rs`
- [x] Create `src/services/short_code.rs` with `ShortCodeService`
- [x] Move `generate_short_code()` function and alphabet constants to the service
- [x] Move `hours_from_now()` helper to appropriate utilities module
- [x] Add unit tests for short code generation
- [x] `cargo test` passes all tests

**Status:** ✅ Completed - Commit 49ef761

### US-009: Create utilities module for shared helpers
**Description:** As a developer, I want a centralized utilities module for shared helper functions that don't belong in any specific domain module.

**Acceptance Criteria:**
- [x] Create `src/util.rs` or `src/utils/` directory
- [x] Move generic helper functions from routes
- [x] Add documentation for each utility function
- [x] `cargo check` passes with no errors

**Status:** ✅ Completed - Commit bf02c9b

### US-010: Update CLAUDE.md with new file structure
**Description:** As a developer, I want the documentation to reflect the new codebase organization so future contributors can understand the updated structure.

**Acceptance Criteria:**
- [x] Update `CLAUDE.md` with new file structure
- [x] Update the architecture diagram if needed
- [x] Update the "Key Files and Their Responsibilities" table
- [x] Ensure all file paths in documentation are accurate

**Status:** ✅ Completed - Commit 7b91271

## Functional Requirements

- FR-1: All existing API endpoints must continue to work with identical behavior
- FR-2: All existing environment variables must remain supported
- FR-3: No database schema changes
- FR-4: No changes to HTTP request/response formats
- FR-5: All existing tests must pass without modification
- FR-6: Build process (`cargo build`, `cargo test`) must work identically
- FR-7: No new dependencies added
- FR-8: Module visibility (`pub` vs `pub(crate)`) must be intentional

## Non-Goals (Out of Scope)

- No changes to the HTTP API contract
- No changes to database schema or migrations
- No new features or functionality
- No performance optimizations (unless a side effect of refactoring)
- No changes to deployment or configuration processes
- No adoption of new architectural patterns (CQRS, Hexagonal, etc.)
- No changes to authentication logic or JWT handling

## Design Considerations

### Proposed New File Structure

```
src/
├── main.rs                 # CLI entry point only
├── server.rs               # Server startup, shutdown, worker spawning
├── state.rs                # AppState definition
├── lib.rs                  # Library exports (if applicable)
├── config/
│   ├── mod.rs              # Config struct, from_env(), re-exports
│   ├── server.rs           # ServerConfig
│   ├── database.rs         # DatabaseConfig
│   ├── cache.rs            # CacheConfig
│   ├── rate_limit.rs       # RateLimitConfig
│   ├── url.rs              # UrlConfig (code length, etc.)
│   └── auth.rs             # AuthConfig
├── routes/
│   ├── mod.rs              # Module exports
│   ├── router.rs           # Router creation, middleware setup
│   ├── handlers.rs         # Handler type definitions
│   ├── health.rs           # Health check endpoint
│   ├── url_handlers.rs     # URL create/resolve/info
│   ├── auth_handlers.rs    # Login endpoint
│   ├── admin_handlers.rs   # Stats, list, delete
│   ├── types.rs            # Route-specific request/response types
│   └── helpers.rs          # Shared handler utilities
├── services/
│   ├── mod.rs
│   └── short_code.rs       # Short code generation service
├── handlers/               # Alternative: domain-based handler modules
│   ├── mod.rs
│   ├── url.rs
│   ├── auth.rs
│   └── admin.rs
├── util.rs                 # or utils/ with submodules
├── models.rs               # (keep as-is)
├── db.rs                   # (keep as-is)
├── cache.rs                # (keep as-is)
├── auth.rs                 # (keep as-is)
├── jobs.rs                 # (keep as-is)
├── middleware.rs           # (keep as-is)
├── middleware_impls.rs     # (keep as-is)
├── error.rs                # (keep as-is)
└── admin.rs                # NEW: CLI admin command handlers
```

### Module Organization Principles

1. **Single Responsibility**: Each module has one clear purpose
2. **Low Coupling**: Minimize cross-module dependencies
3. **High Cohesion**: Related functionality stays together
4. **Clear Public API**: Each module exposes only what's needed externally
5. **Flat Preferred**: Don't create subdirectories unless the module has >3 files

## Technical Considerations

### Module Visibility Strategy

- Use `pub(crate)` for items only needed within the crate
- Use `pub` only for the external library API (if exposing as a library)
- Re-export commonly used types at appropriate levels for convenience

### Import Organization

- Group imports: std, external crates, internal modules
- Prefer `use crate::module::item` over relative paths for clarity
- Consider `prelude.rs` pattern if many modules share common imports

### Testing Strategy

- Keep unit tests alongside the code they test (in `#[cfg(test)]` modules)
- Consider `tests/` directory for integration tests that span multiple modules
- Ensure refactoring doesn't break test coverage

### Incremental Migration Path

1. Create new module structure alongside existing code
2. Move code one module at a time
3. Update imports incrementally
4. Delete old code only after new structure compiles and tests pass

## Success Metrics

- No single source file exceeds 300 lines (excluding generated code and tests)
- `cargo build --release` succeeds without warnings
- All existing tests pass
- `cargo clippy` produces no new warnings
- Codebase can be built and tested with a single `cargo test` run
- New file structure is intuitive to developers unfamiliar with the codebase

## Open Questions

1. Should `handlers/` be organized by domain (url/auth/admin) or by HTTP method (get/post/delete)?
   - **Recommendation**: By domain - more aligned with feature boundaries

2. Should we use `lib.rs` to expose a reusable library, or keep everything in `main.rs`?
   - **Recommendation**: Keep `main.rs` as binary entry point only - no library needed for this service

3. Should short code generation be a "service" with state, or just a pure function module?
   - **Recommendation**: Pure function module - no state needed currently

4. How should we handle the `middleware_impls.rs` split?
   - **Recommendation**: Merge into `middleware.rs` if small, or split by functionality if larger
