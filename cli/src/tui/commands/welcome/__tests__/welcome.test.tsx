import React from 'react';
import { describe, it, expect, vi } from 'vitest';
import { render } from 'ink-testing-library';
import { Welcome } from '../Welcome.js';
import { ANVIL_LOGO, VALUE_PROPOSITION, QUICK_START_OPTIONS } from '../content.js';

describe('Welcome component', () => {
  it('renders the Anvil logo', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('Anvil');
  });

  it('renders the value proposition', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('AI-generated code changes');
    expect(lastFrame()).toContain('safe for production');
  });

  it('renders Quick Start header', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('QUICK START');
  });

  it('renders all quick start options', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('Initialise Anvil');
    expect(lastFrame()).toContain('Run diagnostics');
    expect(lastFrame()).toContain('View commands');
    expect(lastFrame()).toContain('Skip');
  });

  it('renders skip welcome hint', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('ANVIL_SKIP_WELCOME=1');
  });

  it('shows first option as selected by default', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    expect(lastFrame()).toContain('▸');
    expect(lastFrame()).toContain('Initialise Anvil');
  });

  it('handles Enter key press', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    const frame = lastFrame();
    expect(frame).toContain('Initialise Anvil');
    expect(frame).toContain('Enter select');
  });

  it('navigates down with arrow key', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { stdin, lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    stdin.write('\x1B[B');

    expect(lastFrame()).toContain('Run diagnostics');
  });

  it('navigates down with j key', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { stdin, lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    stdin.write('j');

    const frame = lastFrame();
    expect(frame).toContain('Run diagnostics');
  });

  it('navigates up with k key', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { stdin, lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    stdin.write('k');

    const frame = lastFrame();
    expect(frame).toContain('Skip');
  });

  it('wraps around when navigating past last option', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { stdin, lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    for (let i = 0; i < QUICK_START_OPTIONS.length; i++) {
      stdin.write('j');
    }

    expect(lastFrame()).toContain('Initialise Anvil');
  });

  it('wraps around when navigating before first option', () => {
    const onSelect = vi.fn();
    const onQuit = vi.fn();
    const { stdin, lastFrame } = render(<Welcome onSelect={onSelect} onQuit={onQuit} />);

    stdin.write('k');

    expect(lastFrame()).toContain('Skip');
  });
});

describe('Welcome content', () => {
  it('ANVIL_LOGO is block art', () => {
    expect(ANVIL_LOGO).toContain('█');
    expect(ANVIL_LOGO.length).toBeGreaterThan(30);
  });

  it('VALUE_PROPOSITION uses UK English', () => {
    expect(VALUE_PROPOSITION).not.toContain('organize');
    expect(VALUE_PROPOSITION).not.toContain('realize');
  });

  it('QUICK_START_OPTIONS has exactly 4 options', () => {
    expect(QUICK_START_OPTIONS).toHaveLength(4);
  });

  it('QUICK_START_OPTIONS includes init command', () => {
    const initOption = QUICK_START_OPTIONS.find((o) => o.key === 'init');
    expect(initOption).toBeDefined();
    expect(initOption?.command).toBe('anvil init');
  });

  it('QUICK_START_OPTIONS includes doctor command', () => {
    const doctorOption = QUICK_START_OPTIONS.find((o) => o.key === 'doctor');
    expect(doctorOption).toBeDefined();
    expect(doctorOption?.command).toBe('anvil doctor');
  });

  it('QUICK_START_OPTIONS includes help command', () => {
    const helpOption = QUICK_START_OPTIONS.find((o) => o.key === 'help');
    expect(helpOption).toBeDefined();
    expect(helpOption?.command).toBe('anvil --help');
  });

  it('QUICK_START_OPTIONS includes skip option with no command', () => {
    const skipOption = QUICK_START_OPTIONS.find((o) => o.key === 'skip');
    expect(skipOption).toBeDefined();
    expect(skipOption?.command).toBeNull();
  });
});
