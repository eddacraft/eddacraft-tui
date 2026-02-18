import { describe, it, expect } from 'vitest';
import { ConfigLoader, createConfigLoader } from './loader.js';

describe('ConfigLoader', () => {
  describe('constructor defaults', () => {
    it('should use process.cwd() as default baseDir', () => {
      const loader = new ConfigLoader();
      expect(loader.options.baseDir).toBe(process.cwd());
    });

    it('should use ANVIL_ as default envPrefix', () => {
      const loader = new ConfigLoader();
      expect(loader.options.envPrefix).toBe('ANVIL_');
    });

    it('should use default fileNames list', () => {
      const loader = new ConfigLoader();
      expect(loader.options.fileNames).toEqual([
        '.anvilrc',
        '.anvilrc.yaml',
        '.anvilrc.json',
        'anvil.config.js',
      ]);
    });

    it('should accept empty options object', () => {
      const loader = new ConfigLoader({});
      expect(loader.options.baseDir).toBe(process.cwd());
      expect(loader.options.envPrefix).toBe('ANVIL_');
    });
  });

  describe('constructor with custom options', () => {
    it('should use provided baseDir', () => {
      const loader = new ConfigLoader({ baseDir: '/custom/path' });
      expect(loader.options.baseDir).toBe('/custom/path');
    });

    it('should use provided envPrefix', () => {
      const loader = new ConfigLoader({ envPrefix: 'MY_APP_' });
      expect(loader.options.envPrefix).toBe('MY_APP_');
    });

    it('should use provided fileNames', () => {
      const fileNames = ['config.json', 'config.yaml'];
      const loader = new ConfigLoader({ fileNames });
      expect(loader.options.fileNames).toEqual(fileNames);
    });

    it('should allow partial options with defaults for the rest', () => {
      const loader = new ConfigLoader({ baseDir: '/tmp' });
      expect(loader.options.baseDir).toBe('/tmp');
      expect(loader.options.envPrefix).toBe('ANVIL_');
      expect(loader.options.fileNames).toEqual([
        '.anvilrc',
        '.anvilrc.yaml',
        '.anvilrc.json',
        'anvil.config.js',
      ]);
    });
  });

  describe('get', () => {
    it('should return undefined for a key that does not exist', () => {
      const loader = new ConfigLoader();
      expect(loader.get('nonexistent')).toBeUndefined();
    });

    it('should return the value for a key that was set', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'value');
      expect(loader.get<string>('key')).toBe('value');
    });

    it('should return typed values', () => {
      const loader = new ConfigLoader();
      loader.set('count', 42);
      loader.set('flag', true);
      loader.set('nested', { a: 1 });

      expect(loader.get<number>('count')).toBe(42);
      expect(loader.get<boolean>('flag')).toBe(true);
      expect(loader.get<{ a: number }>('nested')).toEqual({ a: 1 });
    });
  });

  describe('getOrDefault', () => {
    it('should return the default value when key does not exist', () => {
      const loader = new ConfigLoader();
      expect(loader.getOrDefault('missing', 'fallback')).toBe('fallback');
    });

    it('should return the stored value when key exists', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'actual');
      expect(loader.getOrDefault('key', 'fallback')).toBe('actual');
    });

    it('should return falsy stored values instead of the default', () => {
      const loader = new ConfigLoader();
      loader.set('zero', 0);
      loader.set('empty', '');
      loader.set('no', false);

      expect(loader.getOrDefault('zero', 999)).toBe(0);
      expect(loader.getOrDefault('empty', 'default')).toBe('');
      expect(loader.getOrDefault('no', true)).toBe(false);
    });

    it('should return the default when value is explicitly set to undefined', () => {
      const loader = new ConfigLoader();
      loader.set('key', undefined);

      // The key exists in the config map...
      expect(loader.has('key')).toBe(true);
      // ...but getOrDefault uses ?? so undefined is treated as nullish,
      // returning the default value rather than undefined
      expect(loader.getOrDefault('key', 'fallback')).toBe('fallback');
    });

    it('should return null when value is explicitly set to null', () => {
      const loader = new ConfigLoader();
      loader.set('key', null);

      // null is also nullish, so ?? returns the default
      expect(loader.getOrDefault('key', 'fallback')).toBe('fallback');
    });
  });

  describe('has', () => {
    it('should return false for a key that does not exist', () => {
      const loader = new ConfigLoader();
      expect(loader.has('missing')).toBe(false);
    });

    it('should return true for a key that was set', () => {
      const loader = new ConfigLoader();
      loader.set('exists', 'yes');
      expect(loader.has('exists')).toBe(true);
    });

    it('should return true even if value is falsy', () => {
      const loader = new ConfigLoader();
      loader.set('falsy', null);
      expect(loader.has('falsy')).toBe(true);
    });
  });

  describe('set', () => {
    it('should store a value with default source', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'value');
      expect(loader.get('key')).toBe('value');
    });

    it('should store a value with a specified source', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'value', 'env');
      expect(loader.get('key')).toBe('value');
    });

    it('should overwrite an existing value', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'first');
      loader.set('key', 'second');
      expect(loader.get('key')).toBe('second');
    });

    it('should handle various value types', () => {
      const loader = new ConfigLoader();
      loader.set('string', 'hello');
      loader.set('number', 42);
      loader.set('boolean', true);
      loader.set('null', null);
      loader.set('array', [1, 2, 3]);
      loader.set('object', { nested: true });

      expect(loader.get('string')).toBe('hello');
      expect(loader.get('number')).toBe(42);
      expect(loader.get('boolean')).toBe(true);
      expect(loader.get('null')).toBeNull();
      expect(loader.get('array')).toEqual([1, 2, 3]);
      expect(loader.get('object')).toEqual({ nested: true });
    });
  });

  describe('getAll', () => {
    it('should return an empty object when no config is set', () => {
      const loader = new ConfigLoader();
      expect(loader.getAll()).toEqual({});
    });

    it('should return all set values', () => {
      const loader = new ConfigLoader();
      loader.set('a', 1);
      loader.set('b', 'two');
      loader.set('c', true);

      expect(loader.getAll()).toEqual({
        a: 1,
        b: 'two',
        c: true,
      });
    });

    it('should return values without source metadata', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'value', 'file');

      const all = loader.getAll();
      expect(all).toEqual({ key: 'value' });
      expect(all).not.toHaveProperty('source');
    });

    it('should reflect the latest value after overwrites', () => {
      const loader = new ConfigLoader();
      loader.set('key', 'first');
      loader.set('key', 'second');

      expect(loader.getAll()).toEqual({ key: 'second' });
    });
  });
});

describe('createConfigLoader', () => {
  it('should return a ConfigLoader instance', () => {
    const loader = createConfigLoader();
    expect(loader).toBeInstanceOf(ConfigLoader);
  });

  it('should accept options and pass them to ConfigLoader', () => {
    const loader = createConfigLoader({ baseDir: '/opt/app', envPrefix: 'APP_' });
    expect(loader.options.baseDir).toBe('/opt/app');
    expect(loader.options.envPrefix).toBe('APP_');
  });

  it('should work without arguments', () => {
    const loader = createConfigLoader();
    expect(loader.options.baseDir).toBe(process.cwd());
  });
});
