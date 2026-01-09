import { describe, it, expect, vi, beforeEach } from 'vitest';
import { resolvePlanPathOrId } from './plan-resolution.js';
import * as fileIo from './file-io.js';
import { existsSync } from 'fs';

vi.mock('fs');
vi.mock('./file-io.js');

describe('resolvePlanPathOrId', () => {
  beforeEach(() => {
    vi.resetAllMocks();
  });

  describe('plan ID resolution', () => {
    it('should resolve valid plan ID', () => {
      const mockWorkspaceRoot = '/workspace';
      const mockPlanPath = '/workspace/.anvil/plans/aps-abc12345.json';

      vi.mocked(fileIo.getWorkspaceRoot).mockReturnValue(mockWorkspaceRoot);
      vi.mocked(fileIo.findPlanById).mockReturnValue(mockPlanPath);

      const result = resolvePlanPathOrId('aps-abc12345');

      expect(result).toEqual({
        path: mockPlanPath,
        wasId: true,
      });
      expect(fileIo.findPlanById).toHaveBeenCalledWith('aps-abc12345', mockWorkspaceRoot);
    });

    it('should use provided workspace root', () => {
      const customRoot = '/custom/root';
      const mockPlanPath = '/custom/root/.anvil/plans/aps-test1234.json';

      vi.mocked(fileIo.findPlanById).mockReturnValue(mockPlanPath);

      const result = resolvePlanPathOrId('aps-test1234', customRoot);

      expect(result).toEqual({
        path: mockPlanPath,
        wasId: true,
      });
      expect(fileIo.findPlanById).toHaveBeenCalledWith('aps-test1234', customRoot);
      expect(fileIo.getWorkspaceRoot).not.toHaveBeenCalled();
    });

    it('should throw error for non-existent plan ID', () => {
      vi.mocked(fileIo.getWorkspaceRoot).mockReturnValue('/workspace');
      vi.mocked(fileIo.findPlanById).mockReturnValue(null);

      expect(() => {
        resolvePlanPathOrId('aps-notfound');
      }).toThrow("Plan with ID 'aps-notfound' not found");
    });
  });

  describe('file path resolution', () => {
    it('should resolve existing file path', () => {
      const filePath = './my-plan.json';
      vi.mocked(existsSync).mockReturnValue(true);

      const result = resolvePlanPathOrId(filePath);

      expect(result).toEqual({
        path: filePath,
        wasId: false,
      });
      expect(existsSync).toHaveBeenCalledWith(filePath);
    });

    it('should resolve absolute file path', () => {
      const filePath = '/absolute/path/to/plan.json';
      vi.mocked(existsSync).mockReturnValue(true);

      const result = resolvePlanPathOrId(filePath);

      expect(result).toEqual({
        path: filePath,
        wasId: false,
      });
    });

    it('should throw error for non-existent file', () => {
      const filePath = './nonexistent.json';
      vi.mocked(existsSync).mockReturnValue(false);

      expect(() => {
        resolvePlanPathOrId(filePath);
      }).toThrow(`Plan file not found: ${filePath}`);
    });
  });

  describe('edge cases', () => {
    it('should correctly identify plan IDs vs paths starting with "aps-"', () => {
      // Plan ID
      vi.mocked(fileIo.getWorkspaceRoot).mockReturnValue('/workspace');
      vi.mocked(fileIo.findPlanById).mockReturnValue('/workspace/.anvil/plans/aps-test.json');

      const result1 = resolvePlanPathOrId('aps-test1234');
      expect(result1.wasId).toBe(true);

      // File path that happens to start with "aps-" but has a slash
      vi.mocked(existsSync).mockReturnValue(true);
      const result2 = resolvePlanPathOrId('./aps-file.json');
      expect(result2.wasId).toBe(false);
    });
  });
});
