import { describe, it, expect, beforeEach, vi } from 'vitest';
import { registerCommands } from '../index.js';
import { AnvilService } from '../../services/anvilService.js';
import { DiagnosticsManager } from '../../services/diagnostics.js';
import { StatusBarManager } from '../../services/statusBar.js';
import { GateResultsProvider } from '../../providers/gateResultsProvider.js';
import * as vscode from 'vscode';

describe('Command Registration', () => {
  let mockContext: vscode.ExtensionContext;
  let mockAnvilService: AnvilService;
  let mockDiagnosticsManager: DiagnosticsManager;
  let mockStatusBarManager: StatusBarManager;
  let mockGateResultsProvider: GateResultsProvider;

  beforeEach(() => {
    vi.clearAllMocks();

    mockContext = {
      subscriptions: [],
      extensionPath: '/test',
      globalState: { get: vi.fn(), update: vi.fn() },
      workspaceState: { get: vi.fn(), update: vi.fn() },
    };

    mockAnvilService = new AnvilService(mockContext);
    mockDiagnosticsManager = new DiagnosticsManager();
    mockStatusBarManager = new StatusBarManager();
    mockGateResultsProvider = new GateResultsProvider(mockAnvilService);
  });

  it('should register all commands', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    // Should have registered multiple commands
    expect(vscode.commands.registerCommand).toHaveBeenCalled();

    // Check that key commands are registered
    const commandNames = (
      vscode.commands.registerCommand as ReturnType<typeof vi.fn>
    ).mock.calls.map((call) => call[0]);

    expect(commandNames).toContain('anvil.validate');
    expect(commandNames).toContain('anvil.validateCurrentFile');
    expect(commandNames).toContain('anvil.gate');
    expect(commandNames).toContain('anvil.gateCurrentFile');
    expect(commandNames).toContain('anvil.export');
    expect(commandNames).toContain('anvil.refresh');
    expect(commandNames).toContain('anvil.showOutput');
  });

  it('should add command disposables to context subscriptions', () => {
    const initialLength = mockContext.subscriptions.length;

    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    // Should have added disposables for each registered command
    expect(mockContext.subscriptions.length).toBeGreaterThan(initialLength);
  });

  it('should register validate command', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    const calls = (vscode.commands.registerCommand as ReturnType<typeof vi.fn>).mock.calls;
    const validateCall = calls.find((call) => call[0] === 'anvil.validate');

    expect(validateCall).toBeDefined();
    expect(typeof validateCall![1]).toBe('function');
  });

  it('should register gate command', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    const calls = (vscode.commands.registerCommand as ReturnType<typeof vi.fn>).mock.calls;
    const gateCall = calls.find((call) => call[0] === 'anvil.gate');

    expect(gateCall).toBeDefined();
    expect(typeof gateCall![1]).toBe('function');
  });

  it('should register export command', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    const calls = (vscode.commands.registerCommand as ReturnType<typeof vi.fn>).mock.calls;
    const exportCall = calls.find((call) => call[0] === 'anvil.export');

    expect(exportCall).toBeDefined();
    expect(typeof exportCall![1]).toBe('function');
  });

  it('should register refresh command', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    const calls = (vscode.commands.registerCommand as ReturnType<typeof vi.fn>).mock.calls;
    const refreshCall = calls.find((call) => call[0] === 'anvil.refresh');

    expect(refreshCall).toBeDefined();
    expect(typeof refreshCall![1]).toBe('function');
  });

  it('should register showOutput command', () => {
    registerCommands(
      mockContext,
      mockAnvilService,
      mockDiagnosticsManager,
      mockStatusBarManager,
      mockGateResultsProvider
    );

    const calls = (vscode.commands.registerCommand as ReturnType<typeof vi.fn>).mock.calls;
    const showOutputCall = calls.find((call) => call[0] === 'anvil.showOutput');

    expect(showOutputCall).toBeDefined();
    expect(typeof showOutputCall![1]).toBe('function');
  });
});
