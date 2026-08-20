import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  eddaStackSidebar: [
    {
      type: 'doc',
      id: 'overview',
      label: 'Explanation — Overview',
    },
    {
      type: 'category',
      label: 'Explanation — Capabilities',
      collapsed: false,
      items: [
        {
          type: 'doc',
          id: 'components/kindling',
          label: 'Signal Capture (Kindling)',
        },
        {
          type: 'doc',
          id: 'components/ember',
          label: 'Candidate Review (Ember)',
        },
        {
          type: 'doc',
          id: 'components/edda',
          label: 'Canonical Memory (Edda)',
        },
      ],
    },
    {
      type: 'doc',
      id: 'design-principles',
      label: 'Explanation — Design principles',
    },
    {
      type: 'doc',
      id: 'enterprise-questions',
      label: 'Explanation — Enterprise questions',
    },
    {
      type: 'doc',
      id: 'roadmap',
      label: 'Explanation — Capability roadmap',
    },
  ],
};

export default sidebars;
