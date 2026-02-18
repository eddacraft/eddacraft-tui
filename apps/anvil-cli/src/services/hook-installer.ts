/**
 * Hook Installer - Service for managing Git hooks
 *
 * Handles installation, uninstallation, and verification of Git hooks.
 */

import { existsSync, readFileSync, writeFileSync, mkdirSync, unlinkSync, chmodSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createDebugger } from '@eddacraft/anvil-core';

const log = createDebugger('service');

/** Marker comment to identify Anvil-managed hooks */
export const ANVIL_MARKER = '# Anvil-managed hook';

/**
 * Hook configuration
 */
export interface HookConfig {
  name: string;
  scriptPath: string;
  description: string;
}

/**
 * Available hooks
 */
export const AVAILABLE_HOOKS: HookConfig[] = [
  {
    name: 'pre-commit',
    scriptPath: 'pre-commit.sh',
    description: 'Validates planning documents before commit',
  },
  {
    name: 'pre-push',
    scriptPath: 'pre-push.sh',
    description: 'Runs quality gates before push',
  },
];

/**
 * Hook installer service
 */
export class HookInstaller {
  /**
   * Get the path to bundled hook scripts
   */
  private getScriptsPath(): string {
    const currentDir = dirname(fileURLToPath(import.meta.url));
    // Navigate from cli/dist/services to cli/scripts
    const possiblePaths = [
      join(currentDir, '../../scripts'),
      join(currentDir, '../../../scripts'),
      join(currentDir, '../scripts'),
    ];

    for (const path of possiblePaths) {
      if (existsSync(path)) {
        return path;
      }
    }

    throw new Error('Could not find hook scripts directory');
  }

  /**
   * Load hook script content from file or fallback to embedded content
   */
  loadHookScript(hookName: string): string {
    log(`loadHookScript: hookName=${hookName}`);
    try {
      const scriptsPath = this.getScriptsPath();
      const hookConfig = AVAILABLE_HOOKS.find((h) => h.name === hookName);

      if (!hookConfig) {
        throw new Error(`Unknown hook: ${hookName}`);
      }

      const scriptPath = join(scriptsPath, hookConfig.scriptPath);
      if (existsSync(scriptPath)) {
        return readFileSync(scriptPath, 'utf-8');
      }
    } catch {
      // Fallback to embedded scripts if files not found
    }

    // Fallback embedded scripts (for when running from source without build)
    return this.getEmbeddedScript(hookName);
  }

  /**
   * Get embedded script content (fallback when script files are not found)
   */
  private getEmbeddedScript(hookName: string): string {
    const scripts: Record<string, string> = {
      'pre-commit': `#!/bin/sh
# Anvil pre-commit hook
# Validates planning documents before commit

# Find modified plan files
PLAN_FILES=$(git diff --cached --name-only --diff-filter=ACM | grep -E '\\.(md|yaml|yml|json)$' || true)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Validating planning documents..."
  FAILED=0

  while IFS= read -r file; do
    [ -z "$file" ] && continue
    if anvil validate "$file" 2>/dev/null; then
      echo "  [OK] $file"
    else
      echo "  [FAIL] $file"
      FAILED=1
    fi
  done <<EOF
$PLAN_FILES
EOF

  if [ "$FAILED" -ne 0 ]; then
    echo ""
    echo "Commit blocked: one or more plan files failed validation."
    echo "Run 'anvil validate <file>' to see details."
    exit 1
  fi
fi

exit 0
`,
      'pre-push': `#!/bin/sh
# Anvil pre-push hook
# Runs quality gates before push

# Check for ANVIL_SKIP_HOOKS environment variable
if [ -n "$ANVIL_SKIP_HOOKS" ]; then
  echo "Warning: ANVIL_SKIP_HOOKS is set — hook checks are being skipped." >&2
  exit 0
fi

# Find plan files in the repository
PLAN_FILES=$(find . \\( -name "*.md" -path "*/planning/*" \\) -o -name "*-plan.md" -o -name "*-prd.md" 2>/dev/null | head -5)

if [ -n "$PLAN_FILES" ]; then
  echo "Anvil: Running quality gates..."
  FAILED=0

  while IFS= read -r file; do
    [ -z "$file" ] && continue
    if [ -f "$file" ]; then
      echo "  Checking: $file"
      if ! anvil gate "$file" 2>/dev/null; then
        echo "  [FAIL] Gate failed: $file"
        FAILED=1
      fi
    fi
  done <<EOF
$PLAN_FILES
EOF

  if [ "$FAILED" -ne 0 ]; then
    echo ""
    echo "Push blocked: one or more gates failed."
    echo "Run 'anvil gate <file>' to see details."
    echo "To bypass, set ANVIL_SKIP_HOOKS=1"
    exit 1
  fi

  echo "  [OK] All gates passed"
fi

exit 0
`,
    };

    const script = scripts[hookName];
    if (!script) {
      throw new Error(`No embedded script for hook: ${hookName}`);
    }

    return script;
  }

  /**
   * Check if a hook file contains the Anvil marker
   */
  isAnvilManagedHook(hookPath: string): boolean {
    if (!existsSync(hookPath)) return false;
    const content = readFileSync(hookPath, 'utf-8');
    return content.includes(ANVIL_MARKER);
  }

  /**
   * Install a hook
   */
  installHook(workspaceRoot: string, hookName: string, gitHooksDir: string): void {
    log(`installHook: hookName=${hookName} dir=${gitHooksDir}`);
    const hookPath = join(workspaceRoot, gitHooksDir, hookName);
    const hookContent = this.loadHookScript(hookName);

    // Create hooks directory if it doesn't exist
    const hooksDir = join(workspaceRoot, gitHooksDir);
    if (!existsSync(hooksDir)) {
      mkdirSync(hooksDir, { recursive: true });
    }

    // Write hook file with Anvil marker (after shebang if present)
    const markedContent = hookContent.startsWith('#!')
      ? hookContent.replace('\n', `\n${ANVIL_MARKER}\n`)
      : `${ANVIL_MARKER}\n${hookContent}`;
    writeFileSync(hookPath, markedContent, { mode: 0o755 });

    // Make executable
    try {
      chmodSync(hookPath, 0o755);
    } catch {
      // Chmod may not be available on Windows
    }
  }

  /**
   * Uninstall a hook
   */
  uninstallHook(workspaceRoot: string, hookName: string, gitHooksDir: string): boolean {
    log(`uninstallHook: hookName=${hookName} dir=${gitHooksDir}`);
    const hookPath = join(workspaceRoot, gitHooksDir, hookName);

    if (!existsSync(hookPath)) {
      return false;
    }

    // Only remove if it's Anvil-managed
    if (!this.isAnvilManagedHook(hookPath)) {
      throw new Error(`Hook ${hookName} is not managed by Anvil. Remove manually if needed.`);
    }

    unlinkSync(hookPath);
    return true;
  }

  /**
   * Check if a hook is installed
   */
  isHookInstalled(workspaceRoot: string, hookName: string, gitHooksDir: string): boolean {
    const hookPath = join(workspaceRoot, gitHooksDir, hookName);
    return existsSync(hookPath) && this.isAnvilManagedHook(hookPath);
  }

  /**
   * Backup existing hook
   */
  backupExistingHook(workspaceRoot: string, hookName: string, gitHooksDir: string): void {
    const hookPath = join(workspaceRoot, gitHooksDir, hookName);

    if (existsSync(hookPath) && !this.isAnvilManagedHook(hookPath)) {
      const backupPath = `${hookPath}.backup`;
      const content = readFileSync(hookPath, 'utf-8');
      writeFileSync(backupPath, content);
    }
  }
}
