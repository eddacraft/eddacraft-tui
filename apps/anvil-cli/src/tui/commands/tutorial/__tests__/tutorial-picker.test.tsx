import { describe, it, expect } from 'vitest';
import React from 'react';
import { render } from 'ink-testing-library';
import { stripAnsi } from '../../../../tui/__tests__/test-utils.js';
import { TutorialPicker, resolveTutorialKey } from '../components/TutorialPicker.js';
import type { TutorialOption } from '../components/TutorialPicker.js';

const ALL_TUTORIALS: TutorialOption[] = [
  { topic: 'core', description: 'Core tutorial (scan, watch, fix)' },
  { topic: 'policies', description: 'Write custom OPA/Rego rules' },
  { topic: 'architecture', description: 'Define architecture boundaries' },
  { topic: 'drift', description: 'Track architecture drift over time' },
  { topic: 'ci', description: 'Set up CI integration' },
];

describe('TutorialPicker', () => {
  it('shows instruction text with key hint', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={ALL_TUTORIALS} currentTopic="core" />);
    const frame = stripAnsi(lastFrame()!);
    expect(frame).toContain("What's next");
    expect(frame).toContain('q to exit');
  });

  it('shows check icon for completed tutorials', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={ALL_TUTORIALS}
        currentTopic="core"
        completedTopics={['policies', 'drift']}
      />
    );
    const frame = lastFrame()!;
    // Completed tutorials should show ◆ instead of their number
    expect(frame).toContain('◆');
    // Non-completed tutorials still show numbers
    expect(frame).toContain('architecture');
    expect(frame).toContain('ci');
  });

  it('assigns consecutive numbers only to non-completed topics', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={ALL_TUTORIALS}
        currentTopic="core"
        completedTopics={['policies', 'drift']}
      />
    );
    const frame = stripAnsi(lastFrame()!);
    // With policies (index 0) and drift (index 2) completed,
    // architecture should be 1, ci should be 2 (consecutive)
    expect(frame).toContain('1');
    expect(frame).toContain('2');
    // Should NOT contain 3 or 4 since only 2 selectable topics remain
    expect(frame).not.toContain('3');
    expect(frame).not.toContain('4');
  });

  it('excludes current topic from the list', () => {
    const { lastFrame } = render(<TutorialPicker tutorials={ALL_TUTORIALS} currentTopic="drift" />);
    expect(lastFrame()).not.toContain('Track architecture drift');
  });

  it('renders nothing when no tutorials available', () => {
    const { lastFrame } = render(
      <TutorialPicker
        tutorials={[{ topic: 'core', description: 'Only one' }]}
        currentTopic="core"
      />
    );
    expect(lastFrame()).toBe('');
  });
});

describe('resolveTutorialKey', () => {
  it('resolves number key to topic', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '1')).toBe('policies');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '4')).toBe('ci');
  });

  it('returns null for out of range', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '5')).toBeNull();
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '0')).toBeNull();
  });

  it('returns null for non-numeric', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', 'a')).toBeNull();
  });

  it('skips completed topics when resolving keys', () => {
    // With policies completed, key 1 should resolve to architecture (next selectable)
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '1', ['policies'])).toBe('architecture');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '2', ['policies'])).toBe('drift');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '3', ['policies'])).toBe('ci');
    // Key 4 should be out of range (only 3 selectable)
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '4', ['policies'])).toBeNull();
  });

  it('skips multiple completed topics', () => {
    // With policies and drift completed, only architecture and ci remain
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '1', ['policies', 'drift'])).toBe(
      'architecture'
    );
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '2', ['policies', 'drift'])).toBe('ci');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '3', ['policies', 'drift'])).toBeNull();
  });

  it('works with empty completedTopics (backward compatible)', () => {
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '1', [])).toBe('policies');
    expect(resolveTutorialKey(ALL_TUTORIALS, 'core', '4', [])).toBe('ci');
  });
});
