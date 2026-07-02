import * as pulumi from '@pulumi/pulumi';

// Resolve a Pulumi output to its underlying value inside tests. Only safe
// under `pulumi.runtime.setMocks`, where outputs always resolve.
export function outputValue<T>(output: pulumi.Output<T>): Promise<T> {
  return new Promise((resolve) => {
    output.apply((value) => {
      resolve(value);
      return value;
    });
  });
}

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
