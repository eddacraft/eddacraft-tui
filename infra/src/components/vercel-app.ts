import * as vercel from '@pulumiverse/vercel';
import * as pulumi from '@pulumi/pulumi';

export interface VercelAppArgs {
  name: string;
  framework: string;
  rootDirectory: string;
  gitRepo: string;
  domains: string[];
  envVars?: Record<string, pulumi.Input<string>>;
  buildCommand?: string;
  installCommand?: string;
  ignoreCommand?: string;
  extraWatchPaths?: string[];
  /** Only build automatically on pushes to this branch (default: main). */
  productionBranch?: string;
  /** Skip automatic preview deploys for non-production branches (default: false). */
  skipPreviewDeploys?: boolean;
  /**
   * Adopt an already-existing ProjectDomain instead of creating it. Map keys
   * are domain names (must appear in `domains`); values are pulumi import IDs
   * in `<projectId>/<domain>` form. Remove an entry once adoption succeeds —
   * leaving it in is a no-op but Pulumi will warn on subsequent runs.
   */
  domainImports?: Record<string, string>;
}

export class VercelApp extends pulumi.ComponentResource {
  public readonly projectId: pulumi.Output<string>;
  public readonly domainNames: string[];

  constructor(name: string, args: VercelAppArgs, opts?: pulumi.ComponentResourceOptions) {
    super('anvil:vercel:App', name, {}, opts);

    // Default ignore command: skip build when only unrelated files changed
    // cd to repo root first — Vercel may run this from the rootDirectory
    const prodBranch = args.productionBranch ?? 'main';
    if (!/^[\w./-]+$/.test(prodBranch)) {
      throw new Error(
        `Invalid productionBranch "${prodBranch}" — must contain only word characters, dots, slashes, or hyphens`
      );
    }
    if (args.domainImports) {
      // ProjectDomain pulumi import IDs come in `<projectId>/<domain>` or
      // `<teamId>/<projectId>/<domain>` form; reject typos before they fall
      // back to a create-from-scratch path that re-introduces the
      // `domain_already_in_use` drift the caller wanted to avoid.
      const domainImportPattern = /^(team_[A-Za-z0-9]+\/)?prj_[A-Za-z0-9]+\/.+$/;
      for (const [importedDomain, importId] of Object.entries(args.domainImports)) {
        if (!args.domains.includes(importedDomain)) {
          throw new Error(`domainImports key "${importedDomain}" must appear in the domains list`);
        }
        if (!domainImportPattern.test(importId)) {
          throw new Error(
            `domainImports value for "${importedDomain}" must be a Vercel import ID of the form <projectId>/<domain> or <teamId>/<projectId>/<domain>; got "${importId}"`
          );
        }
      }
    }
    const skipFlag = args.skipPreviewDeploys ? '--skip-preview ' : '';
    const branchFlag =
      args.skipPreviewDeploys && prodBranch !== 'main' ? `--prod-branch ${prodBranch} ` : '';
    const gitProductionBranch =
      args.skipPreviewDeploys || args.productionBranch ? prodBranch : undefined;
    const extraArgs = args.extraWatchPaths?.length
      ? ' ' + args.extraWatchPaths.map((p) => `'${p}'`).join(' ')
      : '';
    const defaultIgnoreCommand = `cd $(git rev-parse --show-toplevel) && bash tools/scripts/vercel-ignore-build.sh ${skipFlag}${branchFlag}${args.rootDirectory}${extraArgs}`;

    const project = new vercel.Project(
      name,
      {
        name: args.name,
        framework: args.framework,
        rootDirectory: args.rootDirectory,
        buildCommand: args.buildCommand,
        installCommand: args.installCommand,
        ignoreCommand: args.ignoreCommand ?? defaultIgnoreCommand,
        previewDeploymentsDisabled: args.skipPreviewDeploys || undefined,
        gitRepository: {
          type: 'github',
          repo: args.gitRepo,
          ...(gitProductionBranch ? { productionBranch: gitProductionBranch } : {}),
        },
      },
      { parent: this }
    );

    args.domains.forEach((domain) => {
      const importId = args.domainImports?.[domain];
      new vercel.ProjectDomain(
        `${name}-${domain.replace(/\./g, '-')}`,
        {
          projectId: project.id,
          domain,
        },
        { parent: this, ...(importId ? { import: importId } : {}) }
      );
    });

    if (args.envVars) {
      for (const [key, value] of Object.entries(args.envVars)) {
        // NEXT_PUBLIC_* is read by Next.js at build time and inlined into the
        // client bundle. Vercel does not expose sensitive env vars to the
        // build environment, so marking them sensitive silently breaks the
        // build (the value becomes undefined and any fallback in code wins).
        const sensitive = !key.startsWith('NEXT_PUBLIC_');
        // Vercel rejects two env vars that share key+target (ENV_CONFLICT). The
        // default Pulumi create-before-delete order trips that on any
        // forces-replacement attribute change (e.g. `sensitive`); deleting the
        // old one first costs a brief gap on apply but keeps replacements
        // unblocked.
        new vercel.ProjectEnvironmentVariable(
          `${name}-${key.toLowerCase().replace(/_/g, '-')}`,
          {
            projectId: project.id,
            key,
            value,
            targets: ['production', 'preview'],
            sensitive,
          },
          { parent: this, deleteBeforeReplace: true }
        );
      }
    }

    this.projectId = project.id;
    this.domainNames = args.domains;

    this.registerOutputs({
      projectId: this.projectId,
      domainNames: this.domainNames,
    });
  }
}
