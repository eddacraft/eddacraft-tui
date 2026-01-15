# APS Validation Module

This module provides comprehensive validation for Anvil Plan Specification (APS)
documents.

## Features

- **Schema Validation**: Uses Zod for strict schema validation with TypeScript
  type safety
- **Hash Validation**: Verifies plan integrity (requires crypto module)
- **User-friendly Errors**: Formats validation errors for CLI display
- **Flexible Options**: Support for strict/non-strict modes and different output
  formats

## Usage

### Basic Validation

```typescript
import { validateAPSPlan } from '@anvil/core';

// Validate a plan
const result = await validateAPSPlan(plan);

if (result.valid) {
  console.log('Plan is valid!', result.data);
} else {
  console.error('Validation failed:', result.formattedErrors);
}
```

### Using the Validator Class

```typescript
import { APSValidator } from '@anvil/core';

const validator = new APSValidator();

// Validate with options
const result = await validator.validate(plan, {
  validateHash: true, // Enable hash validation
  strict: true, // Stop on first error
  format: 'cli', // Format errors for CLI display
});

// Quick schema check
const isValid = validator.isSchemaValid(plan);
```

### Setting up Hash Validation

When the crypto module is available, you can enable hash validation:

```typescript
import { APSValidator } from '@anvil/core';
import { generateHash } from '../crypto'; // When available

const validator = new APSValidator();
validator.setHashValidator(generateHash);

// Now hash validation will work
const result = await validator.validate(plan, { validateHash: true });
```

### Error Handling

The module provides structured error information:

```typescript
const result = await validateAPSPlan(plan);

if (!result.valid && result.issues) {
  result.issues.forEach((issue) => {
    console.log(`Error at ${issue.path}: ${issue.message}`);
    console.log(`Code: ${issue.code}`);
  });
}
```

### Validation Result Structure

```typescript
interface ValidationResult {
  valid: boolean; // Whether validation passed
  data?: APSPlan; // Validated plan data (if valid)
  issues?: ValidationIssue[]; // Detailed validation issues
  summary: string; // Human-readable summary
  formattedErrors?: string; // Formatted error display (CLI mode)
}
```

## Error Types

- `ValidationError`: Base error class for validation failures
- `SchemaValidationError`: Schema validation specific errors
- `HashValidationError`: Hash mismatch errors

## Examples

### CLI Integration

```typescript
import { validateAPSPlan } from '@anvil/core';
import { readFileSync } from 'fs';

const planData = JSON.parse(readFileSync('plan.json', 'utf8'));
const result = await validateAPSPlan(planData, { format: 'cli' });

if (!result.valid) {
  console.error(result.summary);
  console.error(result.formattedErrors);
  process.exit(1);
}

console.log('✅ Plan is valid!');
```

### Custom Validation Logic

```typescript
import { APSValidator, type APSPlan } from '@anvil/core';

class CustomValidator extends APSValidator {
  async validate(plan: unknown, options = {}) {
    // First run standard validation
    const result = await super.validate(plan, options);

    // Add custom validation logic
    if (result.valid && result.data) {
      // Example: Ensure plan has at least one change
      if (result.data.proposed_changes.length === 0) {
        result.valid = false;
        result.issues = [
          {
            path: 'proposed_changes',
            message: 'Plan must have at least one proposed change',
            code: 'CUSTOM_NO_CHANGES',
            severity: 'error',
          },
        ];
      }
    }

    return result;
  }
}
```
