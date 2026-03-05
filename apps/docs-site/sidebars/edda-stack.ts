import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  eddaStackSidebar: [
    'overview',
    {
      type: 'category',
      label: 'Capabilities',
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
      label: 'Design Principles',
    },
    {
      type: 'doc',
      id: 'enterprise-questions',
      label: 'Enterprise Questions',
    },
    {
      type: 'doc',
      id: 'roadmap',
      label: 'Capability Roadmap',
    },
  ],
};

export default sidebars;
