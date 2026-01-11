import { describe, it, expect, vi, beforeEach } from 'vitest';
import { resolvePlanPathOrId } from './plan-resolution.js';
import * as fileIo from './file-io.js';

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
    it('should throw error for non-existent file', () => {
      // Test with a path that definitely doesn't exist
      const filePath = './definitely-nonexistent-plan-file-12345.json';

      expect(() => {
        resolvePlanPathOrId(filePath);
      }).toThrow(`Plan file not found: ${filePath}`);
    });
  });

  describe('edge cases', () => {
    it('should correctly identify plan IDs starting with "aps-"', () => {
      // Plan ID
      vi.mocked(fileIo.getWorkspaceRoot).mockReturnValue('/workspace');
      vi.mocked(fileIo.findPlanById).mockReturnValue('/workspace/.anvil/plans/aps-test.json');

      const result = resolvePlanPathOrId('aps-test1234');
      expect(result.wasId).toBe(true);
    });
  });
});
