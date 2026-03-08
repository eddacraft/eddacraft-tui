export { debug, createDebugger, isDebugEnabled } from './debug.js';
export { parseSeverity, type Severity } from './severity.js';
export { sanitizeIdentifier, validatePathWithinRoot, validateRelativePath } from './path-safety.js';
export {
  gitExec,
  gitExecSync,
  gitRevParse,
  gitCurrentBranch,
  gitRemoteUrl,
  gitStatusPorcelain,
  gitStagedFiles,
  gitLastCommitMessage,
  gitLastCommitAuthor,
  gitRevParseSync,
  gitCurrentBranchSync,
  gitStatusPorcelainSync,
  gitLogSync,
  GitOperationError,
  type GitExecOptions,
  type GitExecResult,
} from './git-operations.js';
