/**
 * Mock VS Code API for testing
 * This provides minimal implementations of VS Code types and functions needed for unit tests
 */

import { vi } from 'vitest';

// URI class
export class Uri {
  static file(path: string): Uri {
    return new Uri(path);
  }

  static parse(value: string): Uri {
    return new Uri(value);
  }

  constructor(public fsPath: string) {}

  toString(): string {
    return this.fsPath;
  }
}

// Range class
export class Range {
  constructor(
    public start: Position,
    public end: Position
  ) {}
}

// Position class
export class Position {
  constructor(
    public line: number,
    public character: number
  ) {}
}

// ThemeColor class
export class ThemeColor {
  constructor(public id: string) {}
}

// ThemeIcon class
export class ThemeIcon {
  constructor(
    public id: string,
    public color?: ThemeColor
  ) {}
}

// DiagnosticSeverity enum
export enum DiagnosticSeverity {
  Error = 0,
  Warning = 1,
  Information = 2,
  Hint = 3,
}

// Diagnostic class
export class Diagnostic {
  constructor(
    public range: Range,
    public message: string,
    public severity: DiagnosticSeverity = DiagnosticSeverity.Error
  ) {}

  source?: string;
  code?: string | { value: string; target: Uri };
  relatedInformation?: DiagnosticRelatedInformation[];
}

// DiagnosticRelatedInformation class
export class DiagnosticRelatedInformation {
  constructor(
    public location: Location,
    public message: string
  ) {}
}

// Location class
export class Location {
  constructor(
    public uri: Uri,
    public range: Range
  ) {}
}

// TreeItemCollapsibleState enum
export enum TreeItemCollapsibleState {
  None = 0,
  Collapsed = 1,
  Expanded = 2,
}

// TreeItem class
export class TreeItem {
  label?: string;
  description?: string;
  tooltip?: string;
  iconPath?: ThemeIcon | Uri;
  command?: { command: string; title: string; arguments?: unknown[] };
  contextValue?: string;
  collapsibleState?: TreeItemCollapsibleState;

  constructor(label: string, collapsibleState?: TreeItemCollapsibleState) {
    this.label = label;
    this.collapsibleState = collapsibleState;
  }
}

// StatusBarAlignment enum
export enum StatusBarAlignment {
  Left = 1,
  Right = 2,
}

// StatusBarItem interface
export interface StatusBarItem {
  text: string;
  tooltip?: string;
  backgroundColor?: ThemeColor;
  command?: string;
  show(): void;
  hide(): void;
  dispose(): void;
}

// OutputChannel interface
export interface OutputChannel {
  name: string;
  append(value: string): void;
  appendLine(value: string): void;
  clear(): void;
  show(preserveFocus?: boolean): void;
  hide(): void;
  dispose(): void;
}

// TextDocument interface
export interface TextDocument {
  uri: Uri;
  fileName: string;
  languageId: string;
  getText(range?: Range): string;
}

// TextEditor interface
export interface TextEditor {
  document: TextDocument;
}

// WorkspaceFolder interface
export interface WorkspaceFolder {
  uri: Uri;
  name: string;
  index: number;
}

// DiagnosticCollection interface
export interface DiagnosticCollection {
  name: string;
  set(uri: Uri, diagnostics: Diagnostic[]): void;
  delete(uri: Uri): void;
  clear(): void;
  get(uri: Uri): Diagnostic[] | undefined;
  dispose(): void;
}

// Event type
export type Event<T> = (listener: (e: T) => unknown) => { dispose(): void };

// EventEmitter class
export class EventEmitter<T> {
  private listeners: Array<(e: T) => unknown> = [];

  get event(): Event<T> {
    return (listener: (e: T) => unknown) => {
      this.listeners.push(listener);
      return {
        dispose: () => {
          const index = this.listeners.indexOf(listener);
          if (index > -1) {
            this.listeners.splice(index, 1);
          }
        },
      };
    };
  }

  fire(data: T): void {
    for (const listener of this.listeners) {
      listener(data);
    }
  }

  dispose(): void {
    this.listeners = [];
  }
}

// ExtensionContext interface
export interface ExtensionContext {
  subscriptions: { dispose(): void }[];
  extensionPath: string;
  globalState: {
    get<T>(key: string): T | undefined;
    update(key: string, value: unknown): Thenable<void>;
  };
  workspaceState: {
    get<T>(key: string): T | undefined;
    update(key: string, value: unknown): Thenable<void>;
  };
}

// CancellationToken interface
export interface CancellationToken {
  isCancellationRequested: boolean;
  onCancellationRequested: Event<unknown>;
}

// CodeLens class
export class CodeLens {
  constructor(
    public range: Range,
    public command?: { command: string; title: string; tooltip?: string; arguments?: unknown[] }
  ) {}
}

// Mock implementations
const mockDiagnosticCollections = new Map<string, DiagnosticCollection>();
const mockOutputChannels: OutputChannel[] = [];
const mockStatusBarItems: StatusBarItem[] = [];

// languages namespace
export const languages = {
  createDiagnosticCollection(name?: string): DiagnosticCollection {
    const diagnostics = new Map<string, Diagnostic[]>();
    const collection: DiagnosticCollection = {
      name: name || 'default',
      set: (uri: Uri, diags: Diagnostic[]) => {
        diagnostics.set(uri.toString(), diags);
      },
      delete: (uri: Uri) => {
        diagnostics.delete(uri.toString());
      },
      clear: () => {
        diagnostics.clear();
      },
      get: (uri: Uri) => {
        return diagnostics.get(uri.toString());
      },
      dispose: () => {
        diagnostics.clear();
      },
    };
    mockDiagnosticCollections.set(name || 'default', collection);
    return collection;
  },
  registerCodeLensProvider: vi.fn(),
};

// window namespace
export const window = {
  createOutputChannel(name: string): OutputChannel {
    const lines: string[] = [];
    const channel: OutputChannel = {
      name,
      append: (value: string) => {
        lines.push(value);
      },
      appendLine: (value: string) => {
        lines.push(value + '\n');
      },
      clear: () => {
        lines.length = 0;
      },
      show: vi.fn(),
      hide: vi.fn(),
      dispose: () => {
        lines.length = 0;
      },
    };
    mockOutputChannels.push(channel);
    return channel;
  },
  createStatusBarItem(_alignment: StatusBarAlignment, _priority?: number): StatusBarItem {
    const item: StatusBarItem = {
      text: '',
      tooltip: undefined,
      backgroundColor: undefined,
      command: undefined,
      show: vi.fn(),
      hide: vi.fn(),
      dispose: vi.fn(),
    };
    mockStatusBarItems.push(item);
    return item;
  },
  showInformationMessage: vi.fn(),
  showWarningMessage: vi.fn(),
  showErrorMessage: vi.fn(),
  showQuickPick: vi.fn(),
  createTreeView: vi.fn(),
  activeTextEditor: undefined as TextEditor | undefined,
  onDidChangeActiveTextEditor: vi.fn(),
};

// workspace namespace
export const workspace = {
  getConfiguration: vi.fn().mockReturnValue({
    get: vi.fn().mockReturnValue(true),
  }),
  workspaceFolders: [] as WorkspaceFolder[],
  getWorkspaceFolder: vi.fn(),
  findFiles: vi.fn().mockResolvedValue([]),
  onDidSaveTextDocument: vi.fn(),
  onDidOpenTextDocument: vi.fn(),
  createFileSystemWatcher: vi.fn(),
  textDocuments: [] as TextDocument[],
  asRelativePath: vi.fn((pathOrUri: string | Uri) => {
    if (typeof pathOrUri === 'string') {
      return pathOrUri;
    }
    return pathOrUri.fsPath;
  }),
  openTextDocument: vi.fn(),
};

// commands namespace
export const commands = {
  registerCommand: vi.fn(),
  executeCommand: vi.fn(),
};

// Export all mocks
export const mockHelpers = {
  resetMocks: () => {
    mockDiagnosticCollections.clear();
    mockOutputChannels.length = 0;
    mockStatusBarItems.length = 0;
    vi.clearAllMocks();
  },
  getDiagnosticCollection: (name: string) => mockDiagnosticCollections.get(name),
  getOutputChannels: () => mockOutputChannels,
  getStatusBarItems: () => mockStatusBarItems,
};
