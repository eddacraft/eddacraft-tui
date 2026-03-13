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
}

export class VercelApp extends pulumi.ComponentResource {
  public readonly projectId: pulumi.Output<string>;
  public readonly domainNames: string[];

  constructor(name: string, args: VercelAppArgs, opts?: pulumi.ComponentResourceOptions) {
    super('anvil:vercel:App', name, {}, opts);

    // Default ignore command: skip build when only unrelated files changed
    // cd to repo root first — Vercel may run this from the rootDirectory
    const extraArgs = args.extraWatchPaths?.length ? ` ${args.extraWatchPaths.join(' ')}` : '';
    const defaultIgnoreCommand = `cd $(git rev-parse --show-toplevel) && bash tools/scripts/vercel-ignore-build.sh ${args.rootDirectory}${extraArgs}`;

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
