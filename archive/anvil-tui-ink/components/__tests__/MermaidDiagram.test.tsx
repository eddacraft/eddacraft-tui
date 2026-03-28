import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect } from 'vitest';
import { MermaidDiagram } from '../MermaidDiagram.js';

describe('MermaidDiagram', () => {
  it('renders a simple flowchart as ASCII', () => {
    const { lastFrame } = render(<MermaidDiagram definition={'graph TD\n  A --> B'} />);
    const frame = lastFrame();
    expect(frame).toContain('A');
    expect(frame).toContain('B');
    // Should contain arrow characters (Unicode box drawing)
    expect(frame).toContain('▼');
  });

  it('renders node labels', () => {
    const { lastFrame } = render(
      <MermaidDiagram definition={'graph TD\n  nodeA["My Label"] --> nodeB'} />
    );
    const frame = lastFrame();
    expect(frame).toContain('My Label');
    expect(frame).toContain('nodeB');
  });

  it('falls back to raw definition on invalid mermaid', () => {
    const { lastFrame } = render(<MermaidDiagram definition="this is not valid mermaid" />);
    const frame = lastFrame();
    expect(frame).toContain('this is not valid mermaid');
  });

  it('passes asciiOptions through to renderer', () => {
    const { lastFrame } = render(
      <MermaidDiagram definition={'graph TD\n  A --> B'} asciiOptions={{ useAscii: true }} />
    );
    const frame = lastFrame();
    // ASCII mode uses +---+ borders instead of Unicode ┌───┐
    expect(frame).toContain('+');
    expect(frame).toContain('A');
    expect(frame).toContain('B');
  });
});
