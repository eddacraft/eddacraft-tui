export { AnvilService } from './anvilService.js';
export type {
  ValidationResult,
  ValidationError,
  ValidationWarning,
  GateResult,
  GateDetail,
  GateResults,
} from './anvilService.js';

export { StatusBarManager } from './statusBar.js';
export type { StatusBarState } from './statusBar.js';

export { DiagnosticsManager } from './diagnostics.js';

export { PlanWatcher } from './planWatcher.js';

export { SourceWatcher } from './sourceWatcher.js';

export { EmbeddedAnalysisService, getEmbeddedAnalysisService } from './embeddedAnalysis.js';
export type {
  AnalysisResult,
  AnalysisWarning,
  EmbeddedAnalysisOptions,
} from './embeddedAnalysis.js';
