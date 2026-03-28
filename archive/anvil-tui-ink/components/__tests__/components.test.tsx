import React from 'react';
import { render } from 'ink-testing-library';
import { describe, it, expect } from 'vitest';
import { Container } from '../Container.js';
import { Header } from '../Header.js';
import { Divider } from '../Divider.js';
import { StatusBadge } from '../StatusBadge.js';
import { ProgressBar } from '../ProgressBar.js';
import { Text } from 'ink';

describe('TUI Components', () => {
  describe('Container', () => {
    it('renders children', () => {
      const { lastFrame } = render(
        <Container>
          <Text>Hello</Text>
        </Container>
      );

      expect(lastFrame()).toContain('Hello');
    });

    it('renders with title', () => {
      const { lastFrame } = render(
        <Container title="My Title">
          <Text>Content</Text>
        </Container>
      );

      expect(lastFrame()).toContain('My Title');
      expect(lastFrame()).toContain('Content');
    });
  });

  describe('Header', () => {
    it('renders title in uppercase', () => {
      const { lastFrame } = render(<Header title="Anvil" />);

      expect(lastFrame()).toContain('ANVIL');
    });

    it('renders title with subtitle', () => {
      const { lastFrame } = render(<Header title="Anvil" subtitle="Quality Gates" />);

      expect(lastFrame()).toContain('ANVIL');
      expect(lastFrame()).toContain('Quality Gates');
    });

    it('renders title with version', () => {
      const { lastFrame } = render(<Header title="Anvil" version="1.0.0" />);

      expect(lastFrame()).toContain('ANVIL');
      expect(lastFrame()).toContain('v1.0.0');
    });

    it('renders title with subtitle and version', () => {
      const { lastFrame } = render(
        <Header title="Anvil" subtitle="Quality Gates" version="1.0.0" />
      );

      expect(lastFrame()).toContain('ANVIL');
      expect(lastFrame()).toContain('Quality Gates');
      expect(lastFrame()).toContain('v1.0.0');
    });
  });

  describe('Divider', () => {
    it('renders horizontal line', () => {
      const { lastFrame } = render(<Divider />);

      expect(lastFrame()).toContain('─');
    });

    it('renders with custom character', () => {
      const { lastFrame } = render(<Divider character="=" />);

      expect(lastFrame()).toContain('=');
    });
  });

  describe('StatusBadge', () => {
    it('renders success status', () => {
      const { lastFrame } = render(<StatusBadge status="success" />);

      expect(lastFrame()).toContain('Passed');
      expect(lastFrame()).toContain('◆');
    });

    it('renders error status', () => {
      const { lastFrame } = render(<StatusBadge status="error" />);

      expect(lastFrame()).toContain('Failed');
      expect(lastFrame()).toContain('✖');
    });

    it('renders warning status', () => {
      const { lastFrame } = render(<StatusBadge status="warning" />);

      expect(lastFrame()).toContain('Warning');
    });

    it('renders with custom label', () => {
      const { lastFrame } = render(<StatusBadge status="success" label="All tests passed" />);

      expect(lastFrame()).toContain('All tests passed');
    });

    it('renders running status', () => {
      const { lastFrame } = render(<StatusBadge status="running" />);

      expect(lastFrame()).toContain('Running');
    });

    it('renders skipped status', () => {
      const { lastFrame } = render(<StatusBadge status="skipped" />);

      expect(lastFrame()).toContain('Skipped');
    });
  });

  describe('ProgressBar', () => {
    it('renders with percentage', () => {
      const { lastFrame } = render(<ProgressBar percent={50} />);

      expect(lastFrame()).toContain('50%');
    });

    it('renders with label', () => {
      const { lastFrame } = render(<ProgressBar percent={75} label="Progress" />);

      expect(lastFrame()).toContain('Progress');
      expect(lastFrame()).toContain('75%');
    });

    it('clamps percentage to 0-100', () => {
      const { lastFrame: over } = render(<ProgressBar percent={150} />);
      const { lastFrame: under } = render(<ProgressBar percent={-50} />);

      expect(over()).toContain('100%');
      expect(under()).toContain('0%');
    });

    it('hides percentage when showPercent is false', () => {
      const { lastFrame } = render(<ProgressBar percent={50} showPercent={false} />);

      expect(lastFrame()).not.toContain('%');
    });

    it('renders 100% with success colour indicator', () => {
      const { lastFrame } = render(<ProgressBar percent={100} />);

      expect(lastFrame()).toContain('100%');
    });
  });
});
