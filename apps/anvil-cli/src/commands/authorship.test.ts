import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const readAuthorshipNoteMock = vi.fn();
const listAuthorshipNotesMock = vi.fn();
const getAuthorshipStatsMock = vi.fn();
const getWorkspaceRootMock = vi.fn();

vi.mock('@eddacraft/anvil-core', () => ({
  readAuthorshipNote: readAuthorshipNoteMock,
  listAuthorshipNotes: listAuthorshipNotesMock,
  getAuthorshipStats: getAuthorshipStatsMock,
}));

vi.mock('../utils/file-io.js', () => ({
  getWorkspaceRoot: getWorkspaceRootMock,
}));

describe('authorship command', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    getWorkspaceRootMock.mockReturnValue('/tmp/workspace');
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('should create command with correct name and description', async () => {
    const { createAuthorshipCommand } = await import('./authorship.js');
    const command = createAuthorshipCommand();

    expect(command.name()).toBe('authorship');
    expect(command.description()).toContain('Git Notes');
  });

  it('should register show, list, and stats subcommands', async () => {
    const { createAuthorshipCommand } = await import('./authorship.js');
    const command = createAuthorshipCommand();
    const subcommands = command.commands.map((subcommand) => subcommand.name());

    expect(subcommands).toContain('show');
    expect(subcommands).toContain('list');
    expect(subcommands).toContain('stats');
  });

  it('should output JSON for show subcommand when authorship note exists', async () => {
    readAuthorshipNoteMock.mockResolvedValue({
      attestations: {},
      metadata: {
        prompts: {},
        schema_version: '3.0.0',
        base_commit_sha: '0123456789abcdef',
      },
    });
    const consoleLogSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    const { createAuthorshipCommand } = await import('./authorship.js');
    const command = createAuthorshipCommand();

    await command.parseAsync(['node', 'test', 'show', 'HEAD', '--json']);

    expect(readAuthorshipNoteMock).toHaveBeenCalledWith('HEAD', '/tmp/workspace');
    expect(consoleLogSpy).toHaveBeenCalledWith(expect.stringContaining('"attestations"'));
  });
});
