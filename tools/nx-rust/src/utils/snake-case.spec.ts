import { describe, expect, it } from 'vitest';
import { toSnakeCase } from './snake-case';

describe('toSnakeCase', () => {
  it('converts kebab-case to snake_case', () => {
    expect(toSnakeCase('my-crate')).toBe('my_crate');
  });

  it('converts camelCase to snake_case', () => {
    expect(toSnakeCase('myRustCrate')).toBe('my_rust_crate');
  });

  it('converts PascalCase to snake_case', () => {
    expect(toSnakeCase('MyRustCrate')).toBe('my_rust_crate');
  });

  it('collapses consecutive separators and trims edges', () => {
    expect(toSnakeCase('--my__crate--')).toBe('my_crate');
  });

  it('leaves already-snake names alone', () => {
    expect(toSnakeCase('already_snake')).toBe('already_snake');
  });

  it('handles acronyms adjacent to words', () => {
    expect(toSnakeCase('HTTPServer')).toBe('http_server');
  });
});
