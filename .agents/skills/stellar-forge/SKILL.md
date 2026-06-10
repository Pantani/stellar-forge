```markdown
# stellar-forge Development Patterns

> Auto-generated skill from repository analysis

## Overview
This skill teaches the core development patterns and conventions used in the `stellar-forge` TypeScript codebase. You'll learn about file naming, import/export styles, commit message habits, and how to write and run tests. While no automated workflows were detected, this guide provides suggested commands for common development tasks.

## Coding Conventions

### File Naming
- Use **camelCase** for file names.
  - Example: `stellarCore.ts`, `userManager.test.ts`

### Imports
- Use **relative imports** for referencing modules within the project.
  - Example:
    ```typescript
    import { createStar } from './starFactory';
    ```

### Exports
- Use **named exports** rather than default exports.
  - Example:
    ```typescript
    // starFactory.ts
    export function createStar() { ... }
    ```

    ```typescript
    // anotherFile.ts
    import { createStar } from './starFactory';
    ```

### Commit Messages
- Commit messages are **freeform** (no enforced prefix or format).
- Average commit message length: ~29 characters.

## Workflows

_No automated workflows detected in repository._

### Suggested: Running Tests
**Trigger:** When you want to run the test suite.
**Command:** `/run-tests`

1. Ensure all dependencies are installed.
2. Run your test runner (e.g., `npm test` or `npx tsc && node ...`).
3. Review test results.

### Suggested: Adding a New Module
**Trigger:** When adding a new feature or module.
**Command:** `/add-module`

1. Create a new file using camelCase naming.
2. Implement your logic using named exports.
3. Write a corresponding test file with `.test.ts` suffix.
4. Import your module using a relative path where needed.

## Testing Patterns

- Test files use the pattern: `*.test.*` (e.g., `userManager.test.ts`).
- Testing framework is **unknown** (not detected in repository).
- Place test files alongside the modules they test or in a dedicated test directory.
- Example test file:
  ```typescript
  // userManager.test.ts
  import { createUser } from './userManager';

  describe('createUser', () => {
    it('should create a user with a name', () => {
      const user = createUser('Alice');
      expect(user.name).toBe('Alice');
    });
  });
  ```

## Commands
| Command      | Purpose                                 |
|--------------|-----------------------------------------|
| /run-tests   | Run the test suite                      |
| /add-module  | Scaffold a new module with tests        |
```
