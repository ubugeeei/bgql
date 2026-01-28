/**
 * Input validation based on schema directives.
 *
 * Supports: @email, @minLength, @maxLength, @min, @max, @pattern, @url, @trim, etc.
 */

import { validationError, type ValidationError } from './types';

/**
 * Validation result.
 */
export type ValidationResult =
  | { valid: true; value: unknown }
  | { valid: false; errors: ValidationError[] };

/**
 * Validator function type.
 */
export type ValidatorFn = (
  value: unknown,
  args: Record<string, unknown>,
  fieldName: string
) => ValidationError | null;

/**
 * Built-in validators.
 */
export const validators: Record<string, ValidatorFn> = {
  /**
   * Validates email format.
   */
  email: (value, _args, fieldName) => {
    if (typeof value !== 'string') {
      return validationError(fieldName, '@email', 'Value must be a string');
    }
    // Simple email regex - for production use a more robust solution
    const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
    if (!emailRegex.test(value)) {
      return validationError(fieldName, '@email', 'Invalid email format');
    }
    return null;
  },

  /**
   * Validates minimum string length.
   */
  minLength: (value, args, fieldName) => {
    if (typeof value !== 'string') {
      return validationError(fieldName, '@minLength', 'Value must be a string');
    }
    const min = args.value as number;
    if (value.length < min) {
      return validationError(
        fieldName,
        '@minLength',
        `String must be at least ${min} characters`
      );
    }
    return null;
  },

  /**
   * Validates maximum string length.
   */
  maxLength: (value, args, fieldName) => {
    if (typeof value !== 'string') {
      return validationError(fieldName, '@maxLength', 'Value must be a string');
    }
    const max = args.value as number;
    if (value.length > max) {
      return validationError(
        fieldName,
        '@maxLength',
        `String must be at most ${max} characters`
      );
    }
    return null;
  },

  /**
   * Validates minimum numeric value.
   */
  min: (value, args, fieldName) => {
    if (typeof value !== 'number') {
      return validationError(fieldName, '@min', 'Value must be a number');
    }
    const min = args.value as number;
    if (value < min) {
      return validationError(
        fieldName,
        '@min',
        `Value must be at least ${min}`
      );
    }
    return null;
  },

  /**
   * Validates maximum numeric value.
   */
  max: (value, args, fieldName) => {
    if (typeof value !== 'number') {
      return validationError(fieldName, '@max', 'Value must be a number');
    }
    const max = args.value as number;
    if (value > max) {
      return validationError(
        fieldName,
        '@max',
        `Value must be at most ${max}`
      );
    }
    return null;
  },

  /**
   * Validates against a regex pattern.
   */
  pattern: (value, args, fieldName) => {
    if (typeof value !== 'string') {
      return validationError(fieldName, '@pattern', 'Value must be a string');
    }
    const pattern = new RegExp(args.regex as string);
    if (!pattern.test(value)) {
      const message = (args.message as string) ?? 'Value does not match required pattern';
      return validationError(fieldName, '@pattern', message);
    }
    return null;
  },

  /**
   * Validates URL format.
   */
  url: (value, _args, fieldName) => {
    if (typeof value !== 'string') {
      return validationError(fieldName, '@url', 'Value must be a string');
    }
    try {
      new URL(value);
      return null;
    } catch {
      return validationError(fieldName, '@url', 'Invalid URL format');
    }
  },

  /**
   * Validates maximum array items.
   */
  maxItems: (value, args, fieldName) => {
    if (!Array.isArray(value)) {
      return validationError(fieldName, '@maxItems', 'Value must be an array');
    }
    const max = args.value as number;
    if (value.length > max) {
      return validationError(
        fieldName,
        '@maxItems',
        `Array must have at most ${max} items`
      );
    }
    return null;
  },

  /**
   * Validates minimum array items.
   */
  minItems: (value, args, fieldName) => {
    if (!Array.isArray(value)) {
      return validationError(fieldName, '@minItems', 'Value must be an array');
    }
    const min = args.value as number;
    if (value.length < min) {
      return validationError(
        fieldName,
        '@minItems',
        `Array must have at least ${min} items`
      );
    }
    return null;
  },

  /**
   * Validates that value is positive.
   */
  positive: (value, _args, fieldName) => {
    if (typeof value !== 'number') {
      return validationError(fieldName, '@positive', 'Value must be a number');
    }
    if (value <= 0) {
      return validationError(fieldName, '@positive', 'Value must be positive');
    }
    return null;
  },

  /**
   * Validates that value is non-negative.
   */
  nonNegative: (value, _args, fieldName) => {
    if (typeof value !== 'number') {
      return validationError(fieldName, '@nonNegative', 'Value must be a number');
    }
    if (value < 0) {
      return validationError(fieldName, '@nonNegative', 'Value must be non-negative');
    }
    return null;
  },
};

/**
 * Transformers that modify input values.
 */
export const transformers: Record<string, (value: unknown) => unknown> = {
  /**
   * Trims whitespace from strings.
   */
  trim: (value) => {
    if (typeof value === 'string') {
      return value.trim();
    }
    return value;
  },

  /**
   * Converts string to lowercase.
   */
  lowercase: (value) => {
    if (typeof value === 'string') {
      return value.toLowerCase();
    }
    return value;
  },

  /**
   * Converts string to uppercase.
   */
  uppercase: (value) => {
    if (typeof value === 'string') {
      return value.toUpperCase();
    }
    return value;
  },
};

/**
 * Validation rule.
 */
export interface ValidationRule {
  readonly directive: string;
  readonly args: Record<string, unknown>;
}

/**
 * Field validation configuration.
 */
export interface FieldValidation {
  readonly fieldName: string;
  readonly rules: ValidationRule[];
  readonly transforms: string[];
}

/**
 * Validates a single field.
 */
export function validateField(
  value: unknown,
  config: FieldValidation
): ValidationResult {
  const errors: ValidationError[] = [];
  let transformedValue = value;

  // Apply transforms first
  for (const transform of config.transforms) {
    const transformer = transformers[transform];
    if (transformer) {
      transformedValue = transformer(transformedValue);
    }
  }

  // Skip validation for null/undefined (use @required for that)
  if (transformedValue === null || transformedValue === undefined) {
    return { valid: true, value: transformedValue };
  }

  // Run validators
  for (const rule of config.rules) {
    const validator = validators[rule.directive];
    if (validator) {
      const error = validator(transformedValue, rule.args, config.fieldName);
      if (error) {
        errors.push(error);
      }
    }
  }

  if (errors.length > 0) {
    return { valid: false, errors };
  }

  return { valid: true, value: transformedValue };
}

/**
 * Validates an entire input object.
 */
export function validateInput(
  input: Record<string, unknown>,
  fields: FieldValidation[]
): ValidationResult {
  const errors: ValidationError[] = [];
  const validatedInput: Record<string, unknown> = { ...input };

  for (const field of fields) {
    const value = input[field.fieldName];
    const result = validateField(value, field);

    if (!result.valid) {
      errors.push(...result.errors);
    } else {
      validatedInput[field.fieldName] = result.value;
    }
  }

  if (errors.length > 0) {
    return { valid: false, errors };
  }

  return { valid: true, value: validatedInput };
}

/**
 * Creates a validation middleware for resolvers.
 */
export function createValidationMiddleware(fields: FieldValidation[]) {
  return function validateArgs<TArgs extends Record<string, unknown>>(
    args: TArgs
  ): { valid: true; args: TArgs } | { valid: false; error: ValidationError } {
    const result = validateInput(args, fields);

    if (!result.valid) {
      return { valid: false, error: result.errors[0] };
    }

    return { valid: true, args: result.value as TArgs };
  };
}

/**
 * Registers a custom validator.
 */
export function registerValidator(name: string, fn: ValidatorFn): void {
  validators[name] = fn;
}

/**
 * Registers a custom transformer.
 */
export function registerTransformer(
  name: string,
  fn: (value: unknown) => unknown
): void {
  transformers[name] = fn;
}
