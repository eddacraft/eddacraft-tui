import { describe, expect, it } from 'vitest';
import { spawnSync } from 'node:child_process';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

// @ts-expect-error — plain .mjs module without type declarations.
import { createOutputLine, sslConfigFor } from '../../scripts/admin-key-manage.mjs';

const repoRoot = fileURLToPath(new URL('../../../', import.meta.url));
const scriptPath = join(repoRoot, 'infra/scripts/admin-key-manage.mjs');

const SSL_ON = { rejectUnauthorized: true };

describe('sslConfigFor', () => {
  it('keeps SSL on for remote hosts', () => {
    expect(sslConfigFor('postgres://user:pass@db.example.com:5432/app')).toEqual(SSL_ON);
  });

  it('disables SSL for loopback hostnames without an explicit sslmode', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app')).toBeUndefined();
    expect(sslConfigFor('postgres://user:pass@127.0.0.1:5432/app')).toBeUndefined();
    expect(sslConfigFor('postgres://user@[::1]:5432/app')).toBeUndefined();
  });

  it('matches the hostname, not a substring of the connection string', () => {
    // These all contain "localhost" somewhere other than the hostname and
    // previously disabled SSL via the naive `includes('localhost')` check.
    expect(sslConfigFor('postgres://user:pass@localhost.example.com:5432/app')).toEqual(SSL_ON);
    expect(sslConfigFor('postgres://user:pass@db.example.com:5432/localhost_shadow')).toEqual(
      SSL_ON
    );
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?application_name=localhost-test')
    ).toEqual(SSL_ON);
  });

  it('honours an explicit sslmode=disable on any host', () => {
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?sslmode=disable')
    ).toBeUndefined();
  });

  it('honours an explicit sslmode requesting SSL, even on loopback', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=require')).toEqual(SSL_ON);
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=verify-full')).toEqual(
      SSL_ON
    );
  });

  it('compares sslmode case-insensitively', () => {
    expect(
      sslConfigFor('postgres://user:pass@db.example.com:5432/app?sslmode=DISABLE')
    ).toBeUndefined();
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=Require')).toEqual(SSL_ON);
  });

  it('keeps SSL on for unknown sslmode values', () => {
    expect(sslConfigFor('postgres://user:pass@localhost:5432/app?sslmode=bogus')).toEqual(SSL_ON);
  });

  it('keeps SSL on when the connection string is not URL-parseable', () => {
    expect(sslConfigFor('host=localhost dbname=app')).toEqual(SSL_ON);
  });
});

describe('createOutputLine', () => {
  it('emits the documented snake_case hashed_key field', () => {
    const parsed = JSON.parse(createOutputLine(42, 'abc123')) as Record<string, unknown>;
    expect(parsed).toEqual({ id: 42, hashed_key: 'abc123' });
  });

  it('terminates the line with a newline', () => {
    expect(createOutputLine(1, 'x')).toMatch(/\n$/);
  });
});

describe('direct invocation', () => {
  it('still enforces the usage contract when run as a script', () => {
    const result = spawnSync('node', [scriptPath, 'frobnicate'], { encoding: 'utf8' });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('usage: admin-key-manage.mjs <create|revoke>');
  });

  it('fails fast on missing env before touching any database', () => {
    const env = { ...process.env };
    delete env['DATABASE_URL'];
    const result = spawnSync('node', [scriptPath, 'create'], { encoding: 'utf8', env });
    expect(result.status).toBe(1);
    expect(result.stderr).toContain('missing required env DATABASE_URL');
  });
});
