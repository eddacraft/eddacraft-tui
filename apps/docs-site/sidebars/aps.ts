import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  apsSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Specification',
      collapsed: false,
      items: ['spec/taxonomy', 'spec/file-layout', 'spec/determinism'],
    },
    {
      type: 'category',
      label: 'Schemas',
      collapsed: true,
      items: ['schemas/json-schema', 'schemas/examples'],
    },
    {
      type: 'category',
      label: 'Examples',
      collapsed: false,
      items: ['examples/minimal-plan', 'examples/multi-module'],
    },
    {
      type: 'category',
      label: 'Tooling',
      collapsed: true,
      items: ['tooling/validation'],
    },
  ],
};

export default sidebars;
