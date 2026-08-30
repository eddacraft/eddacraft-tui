import type {
  DeepReadonly,
  DeliverySurface,
  FeatureFlagDefinition,
  ProductCatalogueManifest,
  ProductFeature,
} from '@eddacraft/anvil-contracts';

type ReadonlyFlag = DeepReadonly<FeatureFlagDefinition>;
type ReadonlyCatalogue = DeepReadonly<ProductCatalogueManifest>;
type ReadonlySurface = DeepReadonly<DeliverySurface>;
type ReadonlyFeature = DeepReadonly<ProductFeature>;

function booleanDefaultEnabled(flag: ReadonlyFlag): boolean {
  const variant = flag.variants.find((entry) => entry.key === flag.defaultVariant);
  if (variant === undefined) {
    return false;
  }
  if (typeof variant.value === 'boolean') {
    return variant.value;
  }
  return true;
}

function featureBooleanFlags(
  feature: ReadonlyFeature,
  flagsByKey: ReadonlyMap<string, ReadonlyFlag>
): ReadonlyFlag[] {
  if (feature.flagLinkage.disposition === 'unflagged') {
    return [];
  }
  return feature.flagLinkage.flagKeys.flatMap((key) => {
    const flag = flagsByKey.get(key);
    return flag !== undefined && flag.valueType === 'boolean' ? [flag] : [];
  });
}

/**
 * CLI invocation default for FLAGCAT-019 / ADR-136.
 *
 * Unflagged features are on. Linked boolean flags whose default variant is
 * false turn a CLI-only feature off (the clap command is an incantation).
 * Features that also ship on another host stay on: the flag is a contract on
 * that other host, not a clap refusal.
 */
export function cliInvocationDefaultOn(
  surface: ReadonlySurface,
  catalogue: ReadonlyCatalogue,
  flagsByKey: ReadonlyMap<string, ReadonlyFlag>
): boolean {
  if (surface.locator.kind !== 'cli') {
    return true;
  }
  const feature = catalogue.productFeatures.find((entry) => entry.key === surface.featureKey);
  if (feature === undefined) {
    return false;
  }
  const booleanFlags = featureBooleanFlags(feature, flagsByKey);
  if (booleanFlags.length === 0 || booleanFlags.every(booleanDefaultEnabled)) {
    return true;
  }
  const hosts = new Set(
    catalogue.deliverySurfaces
      .filter((entry) => entry.featureKey === feature.key)
      .map((entry) => entry.locator.kind)
  );
  return hosts.size > 1;
}

export function listedImpliesOnViolations(
  catalogue: ReadonlyCatalogue,
  flags: readonly ReadonlyFlag[]
): string[] {
  const flagsByKey = new Map(flags.map((flag) => [flag.key, flag]));
  return catalogue.deliverySurfaces
    .filter(
      (surface) =>
        surface.listed &&
        surface.locator.kind === 'cli' &&
        !cliInvocationDefaultOn(surface, catalogue, flagsByKey)
    )
    .map((surface) => surface.key);
}
