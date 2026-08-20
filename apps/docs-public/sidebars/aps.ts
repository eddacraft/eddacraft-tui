import type { SidebarsConfig } from '@docusaurus/plugin-content-docs';

const sidebars: SidebarsConfig = {
  apsSidebar: [
    // `Start here` and `How-to guides` restore six pages that were published
    // but unreachable from this navigation: they were listed only by the
    // retired `apps/docs-site` rollback host, so the public APS site shipped a
    // specification with no install or getting-started page on nav. The
    // completeness gate could not see it while docs-site's sidebar satisfied
    // the check. Grouping mirrors the structure the pages were authored into.
    {
      type: 'category',
      label: 'Start here',
      collapsed: false,
      items: ['overview', 'getting-started', 'workflow', 'terminology'],
    },
    {
      type: 'category',
      label: 'How-to guides',
      collapsed: false,
      items: ['installation', 'guides/ai-agents', 'guides/monorepo'],
    },
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
