import React from 'react';
import { render } from 'ink';
import { TemplateBrowser, type TemplateBrowserResult } from './TemplateBrowser.js';
import type { Template } from '../../../services/template-loader.js';

export { TemplateBrowser, type TemplateBrowserResult };

export async function showTemplateBrowser(
  templates: Template[]
): Promise<TemplateBrowserResult | null> {
  return new Promise((resolve) => {
    const instance = render(
      React.createElement(TemplateBrowser, {
        templates,
        onSelect: (result: TemplateBrowserResult) => {
          resolve(result);
        },
        onCancel: () => {
          resolve(null);
        },
      })
    );

    instance.waitUntilExit().then(() => {
      resolve(null);
    });
  });
}
