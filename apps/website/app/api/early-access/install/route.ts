import { NextResponse } from 'next/server';

const INSTALL_COMMAND = 'brew install eddacraft/tap/anvil';
const VERIFY_TIMEOUT_MS = 8_000;

interface VerifyResponse {
  valid?: boolean;
  isEdict?: boolean;
}

function parseVerifyResponse(value: unknown): VerifyResponse | null {
  if (typeof value !== 'object' || value === null) {
    return null;
  }
  const valid = 'valid' in value ? value.valid : undefined;
  const isEdict = 'isEdict' in value ? value.isEdict : undefined;
  if (valid !== undefined && typeof valid !== 'boolean') {
    return null;
  }
  if (isEdict !== undefined && typeof isEdict !== 'boolean') {
    return null;
  }
  return {
    ...(typeof valid === 'boolean' ? { valid } : {}),
    ...(typeof isEdict === 'boolean' ? { isEdict } : {}),
  };
}

function getAccessKey(body: unknown): string {
  if (!body || typeof body !== 'object' || !('accessKey' in body)) return '';

  const accessKey = (body as { accessKey?: unknown }).accessKey;
  return typeof accessKey === 'string' ? accessKey.trim() : '';
}

export async function POST(request: Request) {
  const body = (await request.json().catch(() => null)) as unknown;
  const accessKey = getAccessKey(body);

  if (!accessKey) {
    return NextResponse.json({ error: 'missing_key' }, { status: 400 });
  }

  const apiBase = (process.env.NEXT_PUBLIC_API_URL ?? 'https://api.eddacraft.ai').replace(
    /\/+$/,
    ''
  );

  const response = await fetch(`${apiBase}/api/v1/auth/verify`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ token: accessKey }),
    cache: 'no-store',
    signal: AbortSignal.timeout(VERIFY_TIMEOUT_MS),
  }).catch(() => null);

  if (!response) {
    return NextResponse.json({ error: 'access_service_unavailable' }, { status: 503 });
  }

  if (!response.ok) {
    const status = response.status >= 500 ? 503 : 401;
    return NextResponse.json(
      { error: status === 503 ? 'access_service_unavailable' : 'invalid_key' },
      { status }
    );
  }

  let json: unknown;
  try {
    json = await response.json();
  } catch {
    return NextResponse.json({ error: 'access_service_unavailable' }, { status: 503 });
  }
  const data = parseVerifyResponse(json);
  if (data === null) {
    return NextResponse.json({ error: 'access_service_unavailable' }, { status: 503 });
  }
  if (data.valid !== true || data.isEdict !== true) {
    return NextResponse.json({ error: 'invalid_key' }, { status: 401 });
  }

  return NextResponse.json({ command: INSTALL_COMMAND });
}
