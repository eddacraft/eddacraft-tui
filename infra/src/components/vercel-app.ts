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
}

export class VercelApp extends pulumi.ComponentResource {
  public readonly project: vercel.Project;
  public readonly domains: vercel.ProjectDomain[];

  constructor(name: string, args: VercelAppArgs, opts?: pulumi.ComponentResourceOptions) {
    super('anvil:vercel:App', name, {}, opts);

    this.project = new vercel.Project(
      name,
      {
        name: args.name,
        framework: args.framework,
        rootDirectory: args.rootDirectory,
        buildCommand: args.buildCommand,
        installCommand: args.installCommand,
        gitRepository: {
          type: 'github',
          repo: args.gitRepo,
        },
      },
      { parent: this }
    );

    this.domains = args.domains.map(
      (domain) =>
        new vercel.ProjectDomain(
          `${name}-${domain.replace(/\./g, '-')}`,
          {
            projectId: this.project.id,
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
            projectId: this.project.id,
            key,
            value,
            targets: ['production', 'preview'],
            sensitive: true,
          },
          { parent: this }
        );
      }
    }

    this.registerOutputs({
      projectId: this.project.id,
    });
  }
}
