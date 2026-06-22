import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  kindlingSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Quickstart',
      collapsed: false,
      items: [
        'quickstart/without-claude-code',
        'quickstart/install',
        'quickstart/first-memory',
        'quickstart/automatic-capture',
      ],
    },
    {
      type: 'category',
      label: 'Core Concepts',
      collapsed: false,
      items: [
        'concepts/observations',
        'concepts/capsules',
        'concepts/retrieval',
        'concepts/storage',
      ],
    },
    {
      type: 'category',
      label: 'Adapters',
      collapsed: true,
      items: [
        'adapters/claude-code',
        'adapters/vscode',
        'adapters/opencode',
        'adapters/pocketflow',
        'adapters/custom',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: true,
      items: [
        'reference/cli',
        'reference/config',
        'reference/formats',
        'reference/crates',
        'reference/integrations',
      ],
    },
  ],
};

export default sidebars;
