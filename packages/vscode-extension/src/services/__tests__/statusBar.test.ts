import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { StatusBarManager } from '../statusBar.js';
import * as vscode from 'vscode';

describe('StatusBarManager', () => {
  let statusBarManager: StatusBarManager;

  beforeEach(() => {
    vi.clearAllMocks();
    statusBarManager = new StatusBarManager();
  });

  afterEach(() => {
    statusBarManager.dispose();
  });

  describe('construction', () => {
    it('should create a status bar item on the left', () => {
      expect(statusBarManager.statusBarItem).toBeDefined();
      expect(statusBarManager.statusBarItem.command).toBe('anvil.showOutput');
    });

    it('should initialise with idle state', () => {
      expect(statusBarManager.getState()).toBe('idle');
    });
  });

  describe('show and hide', () => {
    it('should show status bar when showStatusBar config is true', () => {
      const mockConfig = {
        get: vi.fn().mockReturnValue(true),
      };
      (vscode.workspace.getConfiguration as ReturnType<typeof vi.fn>).mockReturnValue(mockConfig);

      statusBarManager.show();

      expect(statusBarManager.statusBarItem.show).toHaveBeenCalled();
    });

    it('should not show status bar when showStatusBar config is false', () => {
      const mockConfig = {
        get: vi.fn().mockReturnValue(false),
      };
      (vscode.workspace.getConfiguration as ReturnType<typeof vi.fn>).mockReturnValue(mockConfig);

      statusBarManager.show();

      expect(statusBarManager.statusBarItem.show).not.toHaveBeenCalled();
    });

    it('should hide status bar', () => {
      statusBarManager.hide();
      expect(statusBarManager.statusBarItem.hide).toHaveBeenCalled();
    });
  });

  describe('state management', () => {
    it('should set idle state correctly', () => {
      statusBarManager.setIdle();

      expect(statusBarManager.getState()).toBe('idle');
      expect(statusBarManager.statusBarItem.text).toBe('$(shield) Anvil');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Anvil - Click to show output');
      expect(statusBarManager.statusBarItem.backgroundColor).toBeUndefined();
    });

    it('should set validating state correctly', () => {
      statusBarManager.setValidating('test.md');

      expect(statusBarManager.getState()).toBe('validating');
      expect(statusBarManager.statusBarItem.text).toBe('$(loading~spin) Anvil: Validating...');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Validating test.md');
    });

    it('should set validating state without filename', () => {
      statusBarManager.setValidating();

      expect(statusBarManager.getState()).toBe('validating');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Validating plan...');
    });

    it('should set running gates state correctly', () => {
      statusBarManager.setRunningGates('test.md');

      expect(statusBarManager.getState()).toBe('running-gates');
      expect(statusBarManager.statusBarItem.text).toBe('$(loading~spin) Anvil: Running gates...');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Running quality gates on test.md');
    });

    it('should set running gates state without filename', () => {
      statusBarManager.setRunningGates();

      expect(statusBarManager.getState()).toBe('running-gates');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Running quality gates...');
    });

    it('should set success state correctly', () => {
      statusBarManager.setSuccess('All checks passed');

      expect(statusBarManager.getState()).toBe('success');
      expect(statusBarManager.statusBarItem.text).toBe('$(check) Anvil: Passed');
      expect(statusBarManager.statusBarItem.tooltip).toBe('All checks passed');
      expect(statusBarManager.statusBarItem.backgroundColor).toBeUndefined();
    });

    it('should set success state with default message', () => {
      statusBarManager.setSuccess();

      expect(statusBarManager.statusBarItem.tooltip).toBe('All checks passed');
    });

    it('should set error state correctly', () => {
      statusBarManager.setError('Validation failed');

      expect(statusBarManager.getState()).toBe('error');
      expect(statusBarManager.statusBarItem.text).toBe('$(error) Anvil: Failed');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Validation failed');
      expect(statusBarManager.statusBarItem.backgroundColor).toBeDefined();
    });

    it('should set error state with default message', () => {
      statusBarManager.setError();

      expect(statusBarManager.statusBarItem.tooltip).toBe('Validation or gate check failed');
    });

    it('should set warning state correctly', () => {
      statusBarManager.setWarning('Minor issues found');

      expect(statusBarManager.getState()).toBe('warning');
      expect(statusBarManager.statusBarItem.text).toBe('$(warning) Anvil: Warning');
      expect(statusBarManager.statusBarItem.tooltip).toBe('Minor issues found');
      expect(statusBarManager.statusBarItem.backgroundColor).toBeDefined();
    });

    it('should set warning state with default message', () => {
      statusBarManager.setWarning();

      expect(statusBarManager.statusBarItem.tooltip).toBe('Validation passed with warnings');
    });
  });

  describe('success timeout', () => {
    it('should reset to idle after 5 seconds on success', () => {
      vi.useFakeTimers();

      statusBarManager.setSuccess('Success!');
      expect(statusBarManager.getState()).toBe('success');

      vi.advanceTimersByTime(5000);

      expect(statusBarManager.getState()).toBe('idle');

      vi.useRealTimers();
    });

    it('should clear existing timeout when setting new state', () => {
      vi.useFakeTimers();

      statusBarManager.setSuccess('First');
      vi.advanceTimersByTime(2000);

      statusBarManager.setError('Error');
      expect(statusBarManager.getState()).toBe('error');

      vi.advanceTimersByTime(5000);
      expect(statusBarManager.getState()).toBe('error'); // Should not have reset to idle

      vi.useRealTimers();
    });
  });

  describe('dispose', () => {
    it('should dispose status bar item and clear timeout', () => {
      vi.useFakeTimers();

      statusBarManager.setSuccess('Success');
      statusBarManager.dispose();

      expect(statusBarManager.statusBarItem.dispose).toHaveBeenCalled();

      // Advance time to ensure timeout doesn't fire
      vi.advanceTimersByTime(10000);

      vi.useRealTimers();
    });
  });
});
