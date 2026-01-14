#!/bin/bash
# Security Guard Hook - Pre-Tool-Use validation
# Blocks dangerous bash commands before execution

set -euo pipefail

TOOL_INPUT="${1:-}"

# Dangerous patterns to block
DANGEROUS_PATTERNS=(
    'rm -rf /'
    'rm -rf /\*'
    'rm -rf \*'
    'rm -rf \./\*'
    'rm -rf \./'
    'rm -rf \.\.'
    'rm -r \*'
    'rm -r \./\*'
    'rm -rf ~'
    'rm -rf \$HOME'
    'sudo rm -rf'
    'chmod 777 /'
    'chmod -R 777 /'
    'mkfs\.'
    '> /etc/'
    'dd if=/dev/zero'
    'dd if=/dev/random'
    ':(){ :|:& };:'
    '/dev/sda'
    '/dev/nvme'
    'curl.*[|] ?sh'
    'curl.*[|] ?bash'
    'wget.*[|] ?sh'
    'wget.*[|] ?bash'
    'eval.*\$\('
    '\bsudo\b.*passwd'
    '\bsudo\b.*shadow'
    'history -c'
    '> /dev/sda'
    '\\.ssh/.*authorized'
    'nc -e'
    'ncat -e'
)

# Check for dangerous patterns
for pattern in "${DANGEROUS_PATTERNS[@]}"; do
    if echo "$TOOL_INPUT" | grep -qiE "$pattern"; then
        echo "{\"decision\": \"block\", \"reason\": \"Security violation: Pattern '$pattern' is not allowed\"}" >&2
        exit 2
    fi
done

# Check for attempts to modify system directories
PROTECTED_DIRS=(
    '/etc'
    '/usr'
    '/bin'
    '/sbin'
    '/boot'
    '/lib'
    '/lib64'
    '/var/log'
    '/root'
)

# Read-only commands that are safe for protected directories
READONLY_COMMANDS='(cat|less|head|tail|ls|find|grep|file|stat|wc|diff|strings|xxd|hexdump|readlink)'

for dir in "${PROTECTED_DIRS[@]}"; do
    # Escape directory for regex (handle forward slashes)
    escaped_dir=$(printf '%s' "$dir" | sed 's/[[\.*^$()+?{|]/\\&/g')
    
    # Check if the protected directory appears in the command
    if echo "$TOOL_INPUT" | grep -qE "$escaped_dir(/|\"|'|[[:space:]]|$)"; then
        # Extract the command being run
        cmd=$(echo "$TOOL_INPUT" | sed -n 's/.*"command"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' 2>/dev/null || echo "$TOOL_INPUT")
        
        # Block if command contains modification operations with the protected dir
        if echo "$cmd" | grep -qE "^[[:space:]]*(sudo[[:space:]]+)?(rm|rmdir|mv|chmod|chown|chgrp|truncate|shred)"; then
            echo "{\"decision\": \"block\", \"reason\": \"Modifying system directory $dir is not allowed\"}" >&2
            exit 2
        fi
        
        # Block writes/redirects to protected directories
        if echo "$cmd" | grep -qE "(>|>>)[[:space:]]*$escaped_dir"; then
            echo "{\"decision\": \"block\", \"reason\": \"Writing to system directory $dir is not allowed\"}" >&2
            exit 2
        fi
        
        # Block cp/install with protected dir as destination (second argument pattern)
        if echo "$cmd" | grep -qE "(cp|install|rsync)[[:space:]].*[[:space:]]$escaped_dir"; then
            echo "{\"decision\": \"block\", \"reason\": \"Copying to system directory $dir is not allowed\"}" >&2
            exit 2
        fi
    fi
done

# All checks passed
exit 0
