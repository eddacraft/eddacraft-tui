import React from 'react';
import { render, type Instance } from 'ink';
import { isTUIAvailable, type TUIDetectionOptions } from './tty-detection.js';

export interface RenderResult {
  instance: Instance;
  cleanup: () => void;
  waitUntilExit: () => Promise<void>;
}

export function renderTUI<P extends object>(
  Component: React.ComponentType<P>,
  props: P,
  options: TUIDetectionOptions = {}
): RenderResult | null {
  if (!isTUIAvailable(options)) {
    return null;
  }

  const instance = render(React.createElement(Component, props));

  return {
    instance,
    cleanup: () => instance.unmount(),
    waitUntilExit: () => instance.waitUntilExit(),
  };
}

export async function renderTUIAndWait<P extends object>(
  Component: React.ComponentType<P>,
  props: P,
  options: TUIDetectionOptions = {}
): Promise<boolean> {
  const result = renderTUI(Component, props, options);

  if (!result) {
    return false;
  }

  await result.waitUntilExit();
  return true;
}
