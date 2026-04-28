export {
  SNAPSHOT_SCHEMA_VERSION,
  SnapshotViolationSchema,
  type SnapshotViolation,
  SnapshotAntiPatternSchema,
  type SnapshotAntiPattern,
  SnapshotSuppressionSchema,
  type SnapshotSuppression,
  SnapshotMetricsSchema,
  type SnapshotMetrics,
  AntiPatternBreakdownSchema,
  type AntiPatternBreakdown,
  HotspotSchema,
  type Hotspot,
  DriftSnapshotSchema,
  type DriftSnapshot,
  SnapshotMetadataSchema,
  type SnapshotMetadata,
  generateSnapshotFilename,
  generateNamedSnapshotFilename,
  parseSnapshotFilename,
  createEmptySnapshot,
  validateSnapshot,
} from './snapshot-schema.js';

export {
  SNAPSHOTS_DIR,
  ensureSnapshotsDir,
  saveSnapshot,
  loadSnapshot,
  listSnapshots,
  deleteSnapshot,
  snapshotExists,
  getLatestSnapshot,
  resolveSnapshotName,
  SnapshotStore,
} from './snapshot-storage.js';

export {
  type CaptureOptions,
  type CaptureContext,
  captureSnapshot,
  SnapshotCaptureService,
  createSnapshotCaptureService,
} from './snapshot-capture.js';

export {
  type ItemChange,
  type MetricChange,
  type MetricsComparison,
  type AntiPatternChange,
  type SnapshotComparison,
  compareSnapshots,
  formatComparisonSummary,
} from './snapshot-compare.js';

export {
  type ReportOptions,
  type DriftReport,
  type ReportSection,
  generateReport,
  formatReportAsText,
  formatReportAsJson,
} from './report-generator.js';
