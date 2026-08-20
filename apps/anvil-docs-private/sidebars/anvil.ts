import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  anvilSidebar: [
    {
      type: 'category',
      label: 'Explanation / how-to — Overview',
      collapsed: false,
      items: ['overview', 'when-to-use', 'beta-testing-guide'],
    },
    {
      type: 'category',
      label: 'Tutorial — Quickstart',
      collapsed: false,
      items: ['quickstart', 'first-project', 'first-gate'],
    },
    {
      type: 'category',
      label: 'Tutorials',
      collapsed: false,
      items: [
        'tutorials/tutorials',
        'tutorials/first-save-caught',
        'tutorials/rust-project',
        'tutorials/architecture',
        'tutorials/drift',
        'tutorials/policies',
      ],
    },
    {
      type: 'category',
      label: 'Explanation — Concepts',
      collapsed: false,
      items: [
        'concepts/evaluation-model',
        'concepts/glossary',
        'concepts/plans',
        'concepts/gates',
        'concepts/baseline',
        'concepts/policy-model',
        'concepts/boundaries',
        'concepts/sessions',
        'concepts/audit-trail',
        'concepts/review-capsules',
      ],
    },
    {
      type: 'category',
      label: 'Reference',
      collapsed: false,
      items: [
        'reference/what-anvil-can-do',
        'reference/checks',
        'reference/config',
        'reference/cli-reference',
        'reference/rule-reference',
        'reference/policy',
        'reference/support-reference',
      ],
    },
    {
      type: 'category',
      label: 'How-to guides',
      collapsed: true,
      items: [
        'guides/solo-dev-flow',
        'guides/team-flow',
        'guides/agent-harness',
        'guides/save-time-validation',
        'guides/insights',
      ],
    },
    {
      type: 'category',
      label: 'How-to / reference — Integrations',
      collapsed: true,
      items: [
        'integrations/github',
        'integrations/vscode',
        'integrations/mcp',
        'integrations/agent-skills',
        'integrations/watch-output',
      ],
    },
    {
      type: 'category',
      label: 'How-to / reference — Operations',
      collapsed: true,
      items: [
        'operations/config',
        'operations/security',
        'operations/troubleshooting',
        'operations/git-hooks',
        'operations/telemetry',
        'operations/uninstall',
      ],
    },
    {
      type: 'category',
      label: 'Reference — Release notes',
      collapsed: true,
      items: ['releases/changelog', 'releases/upgrade-notes'],
    },
  ],
};

export default sidebars;
