import * as vercel from '@pulumiverse/vercel';
import * as pulumi from '@pulumi/pulumi';

export type VercelDeploymentProtection =
  | 'allDeployments'
  | 'standardProtectionNew'
  | 'standardProtection'
  | 'onlyPreviewDeployments'
  | 'none';

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
   * Gate deployments behind Vercel Authentication. Use `allDeployments` to
   * require auth on both production and previews — the shell must use a
   * protection bypass secret to rewrite through the upstream project.
   */
  deploymentProtection?: VercelDeploymentProtection;
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
    const extraArgs = args.extraWatchPaths?.length
      ? ' ' + args.extraWatchPaths.map((p) => `'${p}'`).join(' ')
      : '';
    const fileCheckCommand = `cd $(git rev-parse --show-toplevel) && bash tools/scripts/vercel-ignore-build.sh ${args.rootDirectory}${extraArgs}`;

    // When skipPreviewDeploys is true, only build on the production branch;
    // all other branches exit 0 (skip). Manual dev deploys via `vercel deploy`.
    const defaultIgnoreCommand = args.skipPreviewDeploys
      ? `if [ -n "$VERCEL_GIT_COMMIT_REF" ] && [ "$VERCEL_GIT_COMMIT_REF" != "${prodBranch}" ]; then echo "Skipping non-production branch"; exit 0; fi && ${fileCheckCommand}`
      : fileCheckCommand;

    const project = new vercel.Project(
      name,
      {
        name: args.name,
        framework: args.framework,
        rootDirectory: args.rootDirectory,
        buildCommand: args.buildCommand,
        installCommand: args.installCommand,
        ignoreCommand: args.ignoreCommand ?? defaultIgnoreCommand,
        gitRepository: {
          type: 'github',
          repo: args.gitRepo,
        },
        ...(args.deploymentProtection && {
          vercelAuthentication: { deploymentType: args.deploymentProtection },
        }),
      },
      { parent: this }
    );

    args.domains.forEach(
      (domain) =>
        new vercel.ProjectDomain(
          `${name}-${domain.replace(/\./g, '-')}`,
          {
            projectId: project.id,
            domain,
          },
          { parent: this }
        )
    );

    if (args.envVars) {
      for (const [key, value] of Object.entries(args.envVars)) {
        new vercel.ProjectEnvironmentVariable(
          `${name}-${key.toLowerCase().replace(/_/g, '-')}`,
          {
            projectId: project.id,
            key,
            value,
            targets: ['production', 'preview'],
            sensitive: true,
          },
          { parent: this }
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
