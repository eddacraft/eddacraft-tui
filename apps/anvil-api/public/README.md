Empty by design.

Vercel's `framework: null` mode treats this project as a static site that needs
an output directory. We have no static assets — the only thing deployed is
`api/index.ts` (the Hono wrapper). Without this directory the build refuses with
"missing output directory".

The `framework: null` setting in `vercel.json` is what suppresses Vercel's Nx
auto-detection, which would otherwise deploy `dist/index.js` as a phantom
`λ index` serverless function (it's a Hono `app` instance, not a Vercel handler
— invocations hang forever).
