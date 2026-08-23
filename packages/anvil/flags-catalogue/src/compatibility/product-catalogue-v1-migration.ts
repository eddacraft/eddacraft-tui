import type { ProductCatalogueV1MigrationMap } from '@eddacraft/anvil-contracts';

/**
 * Curated migration authority for the frozen v1 CLI catalogue.
 *
 * Every v1 feature has exactly one legacy CLI delivery identity. This map is
 * intentionally explicit: ownership and locators must never be inferred from
 * mutable display names.
 */
export const productCatalogueV1Migration: ProductCatalogueV1MigrationMap = {
  check: {
    owner: 'RCLI2',
    deliveryKey: 'cli.check',
    locator: { kind: 'cli', commandPath: ['check'] },
  },
  audit: {
    owner: 'RCLI',
    deliveryKey: 'cli.audit',
    locator: { kind: 'cli', commandPath: ['audit'] },
  },
  gate: { owner: 'RCLI', deliveryKey: 'cli.gate', locator: { kind: 'cli', commandPath: ['gate'] } },
  'gate-config': {
    owner: 'RCLI2',
    deliveryKey: 'cli.gate-config',
    locator: { kind: 'cli', commandPath: ['gate-config'] },
  },
  drift: {
    owner: 'DRIFT',
    deliveryKey: 'cli.drift',
    locator: { kind: 'cli', commandPath: ['drift'] },
  },
  architecture: {
    owner: 'ARCH',
    deliveryKey: 'cli.architecture',
    locator: { kind: 'cli', commandPath: ['architecture'] },
  },
  policy: {
    owner: 'POLENG',
    deliveryKey: 'cli.policy',
    locator: { kind: 'cli', commandPath: ['policy'] },
  },
  export: {
    owner: 'RCLI',
    deliveryKey: 'cli.export',
    locator: { kind: 'cli', commandPath: ['export'] },
  },
  baseline: {
    owner: 'MLP2',
    deliveryKey: 'cli.baseline',
    locator: { kind: 'cli', commandPath: ['baseline'] },
  },
  'audit-chain': {
    owner: 'MLP',
    deliveryKey: 'cli.audit-chain',
    locator: { kind: 'cli', commandPath: ['audit-chain'] },
  },
  'l4-validate': {
    owner: 'MLP2',
    deliveryKey: 'cli.l4-validate',
    locator: { kind: 'cli', commandPath: ['l4-validate'] },
  },
  validate: {
    owner: 'RCLI3',
    deliveryKey: 'cli.validate',
    locator: { kind: 'cli', commandPath: ['validate'] },
  },
  'mcp.install': {
    owner: 'MCPX',
    deliveryKey: 'cli.mcp-install',
    locator: { kind: 'cli', commandPath: ['mcp', 'install'] },
  },
  'mcp.serve': {
    owner: 'RMCPF',
    deliveryKey: 'cli.mcp-serve',
    locator: { kind: 'cli', commandPath: ['mcp', 'serve'] },
  },
  'mcp.config': {
    owner: 'MCPX',
    deliveryKey: 'cli.mcp-config',
    locator: { kind: 'cli', commandPath: ['mcp-config'] },
  },
  'dashboard.aps': {
    owner: 'CIB',
    deliveryKey: 'cli.plan-dashboard',
    locator: { kind: 'cli', commandPath: ['plan', 'dashboard'] },
  },
  'dashboard.architecture': {
    owner: 'TDASH',
    deliveryKey: 'cli.dashboard-architecture',
    locator: { kind: 'cli', commandPath: ['dashboard', 'architecture'] },
  },
  'dashboard.drift': {
    owner: 'TDASH',
    deliveryKey: 'cli.dashboard-drift',
    locator: { kind: 'cli', commandPath: ['dashboard', 'drift'] },
  },
  'dashboard.suppressions': {
    owner: 'TDASH',
    deliveryKey: 'cli.dashboard-suppressions',
    locator: { kind: 'cli', commandPath: ['dashboard', 'suppressions'] },
  },
  'dashboard.saved': {
    owner: 'TDASH',
    deliveryKey: 'cli.dashboard-saved',
    locator: { kind: 'cli', commandPath: ['dashboard'] },
  },
  watch: {
    owner: 'DSV',
    deliveryKey: 'cli.watch',
    locator: { kind: 'cli', commandPath: ['watch'] },
  },
  intercept: {
    owner: 'INTD',
    deliveryKey: 'cli.intercept',
    locator: { kind: 'cli', commandPath: ['intercept'] },
  },
  hook: {
    owner: 'GHOOK',
    deliveryKey: 'cli.hook',
    locator: { kind: 'cli', commandPath: ['hook'] },
  },
  hooks: {
    owner: 'GHOOK',
    deliveryKey: 'cli.hooks',
    locator: { kind: 'cli', commandPath: ['hooks'] },
  },
  'admin.operations': {
    owner: 'ADMINCLI',
    deliveryKey: 'cli.admin-operations',
    locator: { kind: 'cli', commandPath: ['admin'] },
  },
  edda: { owner: 'EDDA', deliveryKey: 'cli.edda', locator: { kind: 'cli', commandPath: ['edda'] } },
  capsule: {
    owner: 'GITGOV',
    deliveryKey: 'cli.capsule',
    locator: { kind: 'cli', commandPath: ['capsule'] },
  },
  insights: {
    owner: 'INSIGHTS',
    deliveryKey: 'cli.insights',
    locator: { kind: 'cli', commandPath: ['insights'] },
  },
  kindling: {
    owner: 'KFIT',
    deliveryKey: 'cli.kindling',
    locator: { kind: 'cli', commandPath: ['kindling'] },
  },
  init: { owner: 'RCLI', deliveryKey: 'cli.init', locator: { kind: 'cli', commandPath: ['init'] } },
  ensure: { owner: 'ONSW', deliveryKey: 'cli.ensure', locator: { kind: 'cli', commandPath: [] } },
  start: {
    owner: 'LAUNCH',
    deliveryKey: 'cli.start',
    locator: { kind: 'cli', commandPath: ['start'] },
  },
  welcome: {
    owner: 'RCLI',
    deliveryKey: 'cli.welcome',
    locator: { kind: 'cli', commandPath: ['welcome'] },
  },
  new: { owner: 'RCLI', deliveryKey: 'cli.new', locator: { kind: 'cli', commandPath: ['new'] } },
  wizard: {
    owner: 'RCLI',
    deliveryKey: 'cli.wizard',
    locator: { kind: 'cli', commandPath: ['wizard'] },
  },
  'admin.credential': {
    owner: 'ADMINCLI',
    deliveryKey: 'cli.admin-credential',
    locator: { kind: 'cli', commandPath: ['admin', 'auth'] },
  },
  auth: {
    owner: 'GHCLIAUTH',
    deliveryKey: 'cli.auth',
    locator: { kind: 'cli', commandPath: ['auth'] },
  },
  config: {
    owner: 'WATCHUX',
    deliveryKey: 'cli.config',
    locator: { kind: 'cli', commandPath: ['config'] },
  },
  migrate: {
    owner: 'UCFG',
    deliveryKey: 'cli.migrate',
    locator: { kind: 'cli', commandPath: ['migrate'] },
  },
  update: {
    owner: 'DISTRIB',
    deliveryKey: 'cli.update',
    locator: { kind: 'cli', commandPath: ['update'] },
  },
  uninstall: {
    owner: 'ADOPT',
    deliveryKey: 'cli.uninstall',
    locator: { kind: 'cli', commandPath: ['uninstall'] },
  },
  doctor: {
    owner: 'RCLI',
    deliveryKey: 'cli.doctor',
    locator: { kind: 'cli', commandPath: ['doctor'] },
  },
  version: {
    owner: 'DISTRIB',
    deliveryKey: 'cli.version',
    locator: { kind: 'cli', commandPath: ['version'] },
  },
  licenses: {
    owner: 'RUSTNX',
    deliveryKey: 'cli.licenses',
    locator: { kind: 'cli', commandPath: ['licenses'] },
  },
  tutorial: {
    owner: 'TUTOR',
    deliveryKey: 'cli.tutorial',
    locator: { kind: 'cli', commandPath: ['tutorial'] },
  },
  workspace: {
    owner: 'INTD',
    deliveryKey: 'cli.workspace',
    locator: { kind: 'cli', commandPath: ['workspace'] },
  },
};
