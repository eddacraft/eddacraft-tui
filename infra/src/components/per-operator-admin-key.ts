import * as pulumi from '@pulumi/pulumi';
import * as command from '@pulumi/command';
import * as random from '@pulumi/random';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const scriptPath = path.resolve(__dirname, '../../scripts/admin-key-manage.mjs');

export interface PerOperatorAdminKeyArgs {
  actorEmail: pulumi.Input<string>;
  note: pulumi.Input<string>;
  changeActor: pulumi.Input<string>;
  commitSha: pulumi.Input<string>;
  databaseUrl: pulumi.Input<string>;
  pepper: pulumi.Input<string>;
}

export class PerOperatorAdminKey extends pulumi.ComponentResource {
  public readonly bearerHex: pulumi.Output<string>;
  public readonly actorEmail: pulumi.Output<string>;

  constructor(name: string, args: PerOperatorAdminKeyArgs, opts?: pulumi.ComponentResourceOptions) {
    super('anvil:admin:PerOperatorAdminKey', name, {}, opts);

    // 32 random bytes; bearer the operator will actually present to the API.
    // Held as a Pulumi secret output; recoverable via `pulumi stack output
    // --show-secrets` by anyone with stack access.
    const bearer = new random.RandomBytes(`${name}-bearer`, { length: 32 }, { parent: this });

    this.bearerHex = pulumi.secret(bearer.hex);
    this.actorEmail = pulumi.output(args.actorEmail);

    new command.local.Command(
      `${name}-row`,
      {
        create: `node ${JSON.stringify(scriptPath)} create`,
        delete: `node ${JSON.stringify(scriptPath)} revoke`,
        // Any change to the bearer or actor email must force a revoke + create
        // rather than an in-place update — the row's hashed_key is the primary
        // operator-facing handle, and changing it mid-flight would invalidate
        // a bearer an operator is already using without an audit trail.
        triggers: [this.bearerHex, this.actorEmail],
        environment: {
          DATABASE_URL: pulumi.secret(args.databaseUrl) as pulumi.Output<string>,
          ADMIN_KEY_PEPPER: pulumi.secret(args.pepper) as pulumi.Output<string>,
          BEARER_HEX: this.bearerHex,
          ACTOR_EMAIL: this.actorEmail,
          NOTE: pulumi.output(args.note),
          CHANGE_ACTOR: pulumi.output(args.changeActor),
          COMMIT_SHA: pulumi.output(args.commitSha),
        },
      },
      { parent: this, additionalSecretOutputs: ['stdout', 'environment'] }
    );

    this.registerOutputs({
      bearerHex: this.bearerHex,
      actorEmail: this.actorEmail,
    });
  }
}
