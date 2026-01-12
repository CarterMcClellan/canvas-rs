# Claude Development Guidelines

## Allowed Commands

Pre-approved commands are configured in `.claude/settings.json`. This includes:

- **npm/npx** - install, test, run, playwright
- **cargo** - build, check, clippy, test
- **trunk** - serve, build
- **git** - standard operations (status, diff, add, commit, push, pull, branch, checkout, log, stash)
- **File ops** - ls, mkdir, rm, cat /tmp/*
- **Process management** - lsof, kill (for dev server ports)

## Running Tests

This project uses Playwright for end-to-end testing. Tests are located in `tests/e2e/`.

### Test Commands

```bash
# Run all tests (requires trunk server running on port 8090)
cd tests && npm test

# Run tests with UI for debugging
cd tests && npm run test:ui

# View test report
cd tests && npm run test:report
```

### Before Running Tests

1. Start the trunk server: `trunk serve --port 8090`
2. Ensure dependencies are installed: `cd tests && npm install`

## Sub-Agents

Custom sub-agents are defined in `.claude/agents/`. Available agents:

### `simplify`
Cleans up and simplifies code after implementation. Use after completing a feature or fix:
- Removes commented-out code and debug statements
- Simplifies overly complex logic
- Cleans up unused imports and dead code
- Applies language-specific idioms (Rust `?` operator, etc.)

**Usage**: After finishing implementation, invoke the simplify agent to clean up the code before committing.

## Development Workflow

### After Major Changes

1. **Run tests** to verify nothing is broken
2. **Run the `simplify` agent** to clean up the implementation
3. **Run tests again** to ensure simplifications didn't break anything

The test suite covers:
- Selection (single click, marquee selection)
- Translation (dragging selected shapes)
- Resize handle operations (including inversions/flips)
- Hover states and cursor behavior
- Edge cases (minimum size constraints, reset functionality)
- GPU transform persistence

### Adding New Features

**When implementing a major feature request, add corresponding tests to the test suite.**

Tests should be added to `tests/e2e/resizable-canvas.spec.ts` (or create a new spec file if the feature is distinct enough).

Test helpers are available in:
- `tests/e2e/helpers/canvas-helpers.ts` - Canvas interaction utilities
- `tests/e2e/helpers/assertions.ts` - Common assertions
- `tests/e2e/fixtures/expected-states.ts` - Expected state constants

### Test Writing Guidelines

1. Use descriptive test names with TC-XX prefix for traceability
2. Use the helper functions for common operations (e.g., `drawSelectionRectangle`, `dragHandle`, `clickOnShape`)
3. Assert both visual state and data state where possible
4. Consider edge cases (minimum sizes, inversions, persistence after deselect/reselect)

## Project Structure

- `src/` - Rust source code (Leptos + WASM)
- `tests/e2e/` - Playwright E2E tests
- `static/` - Static assets
- `Trunk.toml` - Trunk build configuration
