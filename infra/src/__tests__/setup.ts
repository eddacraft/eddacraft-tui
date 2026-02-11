import * as pulumi from '@pulumi/pulumi';

export function setupPulumiMocks() {
  pulumi.runtime.setMocks(
    {
      newResource(args: pulumi.runtime.MockResourceArgs): {
        id: string;
        state: Record<string, unknown>;
      } {
        return { id: `${args.name}-mock-id`, state: args.inputs };
      },
      call(args: pulumi.runtime.MockCallArgs): Record<string, unknown> {
        return args.inputs;
      },
    },
    'test-project',
    'test-stack',
    false
  );
}
