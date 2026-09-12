/**
 * Default Win32 named-pipe open + server-identity check.
 *
 * Isolated so Linux tests never execute FFI. Loaded lazily via
 * `createRequire('koffi')`; koffi is an optionalDependency. Missing koffi,
 * access-denied, or a SID mismatch fail closed as `anvil-daemon-wrong-owner`.
 * A missing or busy pipe is `anvil-daemon-unavailable` — no listener is not
 * an ownership failure.
 *
 * Mirrors `connect_owner_only_overlapped_pipe_client` in
 * `crates/anvil-intercept-win32`: `CreateFileW` with Identification SQOS
 * and overlapped I/O, then `GetNamedPipeServerProcessId` on that HANDLE,
 * then the server process TokenUser SID compared to the current-user SID.
 */

import { createRequire } from 'node:module';
import net from 'node:net';

import { type DriverClientError, driverError } from '../errors.js';

const GENERIC_READ = 0x8000_0000;
const GENERIC_WRITE = 0x4000_0000;
const FILE_SHARE_READ = 0x0000_0001;
const FILE_SHARE_WRITE = 0x0000_0002;
const OPEN_EXISTING = 3;
const SECURITY_SQOS_PRESENT = 0x0010_0000;
const SECURITY_IDENTIFICATION = 0x0001_0000;
const FILE_FLAG_OVERLAPPED = 0x4000_0000;
const PROCESS_QUERY_LIMITED_INFORMATION = 0x1000;
const TOKEN_QUERY = 0x0008;
const TOKEN_USER = 1;
const FILE_TYPE_PIPE = 3;
const O_RDWR = 2;
const O_BINARY = 0x8000;
const FULL_SID_PATTERN = /^S-1-\d+(?:-\d+)+$/i;

/** Win32 `GetLastError` values that mean no usable pipe listener. */
const WIN32_ERROR_FILE_NOT_FOUND = 2;
const WIN32_ERROR_PATH_NOT_FOUND = 3;
const WIN32_ERROR_ACCESS_DENIED = 5;
const WIN32_ERROR_PIPE_BUSY = 231;

type Koffi = {
  load: (name: string) => {
    func: (name: string, result: string, args: unknown[]) => (...args: unknown[]) => unknown;
  };
  out: (type: string) => unknown;
  decode: ((value: unknown, type: string) => unknown) & { wstring?: (ptr: unknown) => string };
  address: (value: unknown) => bigint | number;
  alloc: (type: string, length: number) => unknown;
};

let cachedKoffi: Koffi | null | undefined;

function loadKoffi(): Koffi | null {
  if (cachedKoffi !== undefined) {
    return cachedKoffi;
  }
  try {
    const require = createRequire(import.meta.url);
    cachedKoffi = require('koffi') as Koffi;
    return cachedKoffi;
  } catch {
    cachedKoffi = null;
    return null;
  }
}

function isInvalidHandle(koffi: Koffi, handle: unknown): boolean {
  if (handle === null || handle === undefined) {
    return true;
  }
  try {
    const addr = BigInt(koffi.address(handle));
    return addr === 0n || addr === 0xffff_ffff_ffff_ffffn || addr === 0xffff_ffffn;
  } catch {
    return true;
  }
}

function failAuth(message: string): never {
  throw driverError('anvil-daemon-wrong-owner', message);
}

/**
 * Map a `CreateFileW` last-error to the driver-client connect contract.
 * Missing/busy pipes are unavailable (retriable). Access-denied and unknown
 * open failures stay fail-closed as wrong-owner so a squat or ACL miss is
 * never retried as "daemon down".
 */
export function mapWindowsPipeOpenError(pipeName: string, lastError: number): DriverClientError {
  if (
    lastError === WIN32_ERROR_FILE_NOT_FOUND ||
    lastError === WIN32_ERROR_PATH_NOT_FOUND ||
    lastError === WIN32_ERROR_PIPE_BUSY
  ) {
    return driverError(
      'anvil-daemon-unavailable',
      `cannot open named pipe ${pipeName}: Win32 error ${lastError}`
    );
  }
  if (lastError === WIN32_ERROR_ACCESS_DENIED) {
    return driverError(
      'anvil-daemon-wrong-owner',
      `cannot open named pipe ${pipeName}: Win32 error ${lastError}`
    );
  }
  return driverError(
    'anvil-daemon-wrong-owner',
    `cannot open named pipe for server authentication: ${pipeName}`
  );
}

/**
 * Open `pipeName` with Identification SQOS, authenticate the server process
 * SID, and adopt the HANDLE as a paused `net.Socket`.
 */
export function openAuthenticatedWindowsPipe(pipeName: string, currentUserSid: string): net.Socket {
  if (process.platform !== 'win32') {
    failAuth('named-pipe server authentication requires Windows');
  }
  const koffi = loadKoffi();
  if (koffi === null) {
    failAuth('cannot load Win32 FFI (koffi) to authenticate named-pipe server');
  }

  const kernel32 = koffi.load('kernel32.dll');
  const advapi32 = koffi.load('advapi32.dll');
  const ucrt = koffi.load('ucrtbase.dll');

  const CreateFileW = kernel32.func('CreateFileW', 'void *', [
    'str16',
    'uint32',
    'uint32',
    'void *',
    'uint32',
    'uint32',
    'void *',
  ]);
  const GetLastError = kernel32.func('GetLastError', 'uint32', []);
  const CloseHandle = kernel32.func('CloseHandle', 'bool', ['void *']);
  const GetFileType = kernel32.func('GetFileType', 'uint32', ['void *']);
  const GetNamedPipeServerProcessId = kernel32.func('GetNamedPipeServerProcessId', 'bool', [
    'void *',
    koffi.out('uint32 *'),
  ]);
  const OpenProcess = kernel32.func('OpenProcess', 'void *', ['uint32', 'bool', 'uint32']);
  const OpenProcessToken = advapi32.func('OpenProcessToken', 'bool', [
    'void *',
    'uint32',
    koffi.out('void **'),
  ]);
  const GetTokenInformation = advapi32.func('GetTokenInformation', 'bool', [
    'void *',
    'int32',
    'void *',
    'uint32',
    koffi.out('uint32 *'),
  ]);
  const ConvertSidToStringSidW = advapi32.func('ConvertSidToStringSidW', 'bool', [
    'void *',
    koffi.out('void **'),
  ]);
  const LocalFree = kernel32.func('LocalFree', 'void *', ['void *']);
  const openOsfHandle = ucrt.func('_open_osfhandle', 'int', ['void *', 'int']);

  const flags = SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION | FILE_FLAG_OVERLAPPED;
  const handle = CreateFileW(
    pipeName,
    GENERIC_READ | GENERIC_WRITE,
    FILE_SHARE_READ | FILE_SHARE_WRITE,
    null,
    OPEN_EXISTING,
    flags,
    null
  );
  const lastError = GetLastError() as number;
  if (isInvalidHandle(koffi, handle)) {
    throw mapWindowsPipeOpenError(pipeName, lastError);
  }

  let adopted = false;
  const closePipe = (): void => {
    if (adopted) {
      return;
    }
    try {
      CloseHandle(handle);
    } catch {
      // Best-effort; the subsequent throw is the signal.
    }
  };

  try {
    const fileType = GetFileType(handle) as number;
    if (fileType !== FILE_TYPE_PIPE) {
      failAuth(`connected handle is not a named pipe (GetFileType=${fileType})`);
    }

    const pidOut = [0];
    if (!GetNamedPipeServerProcessId(handle, pidOut)) {
      failAuth('GetNamedPipeServerProcessId failed');
    }
    const serverPid = pidOut[0];
    if (serverPid === undefined || serverPid === 0) {
      failAuth('named-pipe server PID is missing');
    }

    const serverSid = processTokenUserSid(
      koffi,
      OpenProcess,
      OpenProcessToken,
      GetTokenInformation,
      ConvertSidToStringSidW,
      LocalFree,
      CloseHandle,
      serverPid
    );
    assertWindowsServerSid(serverSid, currentUserSid);

    const fd = openOsfHandle(handle, O_RDWR | O_BINARY) as number;
    if (fd < 0) {
      failAuth('cannot adopt named-pipe HANDLE as a CRT file descriptor');
    }
    adopted = true;
    const sock = new net.Socket({ fd, readable: true, writable: true });
    sock.pause();
    return sock;
  } catch (err) {
    closePipe();
    if (err instanceof Error && 'code' in err) {
      throw err;
    }
    const message = err instanceof Error ? err.message : String(err);
    failAuth(`named-pipe server authentication failed: ${message}`);
  }
}

function processTokenUserSid(
  koffi: Koffi,
  OpenProcess: (...args: unknown[]) => unknown,
  OpenProcessToken: (...args: unknown[]) => unknown,
  GetTokenInformation: (...args: unknown[]) => unknown,
  ConvertSidToStringSidW: (...args: unknown[]) => unknown,
  LocalFree: (...args: unknown[]) => unknown,
  CloseHandle: (...args: unknown[]) => unknown,
  pid: number
): string {
  const process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid);
  if (isInvalidHandle(koffi, process)) {
    failAuth(`cannot open named-pipe server process ${pid} for token query`);
  }
  try {
    const tokenOut: unknown[] = [null];
    if (!OpenProcessToken(process, TOKEN_QUERY, tokenOut)) {
      failAuth(`cannot open named-pipe server process ${pid} token`);
    }
    const token = tokenOut[0];
    if (token === undefined || isInvalidHandle(koffi, token)) {
      failAuth(`named-pipe server process ${pid} token handle is invalid`);
    }
    try {
      const needed = [0];
      GetTokenInformation(token, TOKEN_USER, null, 0, needed);
      const byteCount = needed[0];
      if (byteCount === undefined || byteCount === 0) {
        failAuth('GetTokenInformation(TokenUser) did not report a buffer size');
      }
      const buffer = koffi.alloc('uint8_t', byteCount);
      const needed2 = [0];
      if (!GetTokenInformation(token, TOKEN_USER, buffer, byteCount, needed2)) {
        failAuth('GetTokenInformation(TokenUser) failed');
      }
      const sid = koffi.decode(buffer, 'void *');
      const sidStringPtr: unknown[] = [null];
      if (!ConvertSidToStringSidW(sid, sidStringPtr)) {
        failAuth('ConvertSidToStringSidW failed for named-pipe server');
      }
      try {
        const raw = sidStringPtr[0];
        const decoded =
          koffi.decode.wstring !== undefined
            ? koffi.decode.wstring(raw)
            : (koffi.decode(raw, 'str16') as string);
        if (typeof decoded !== 'string' || !FULL_SID_PATTERN.test(decoded)) {
          failAuth('named-pipe server SID is not a canonical S-1-… string');
        }
        return decoded;
      } finally {
        if (sidStringPtr[0] !== null && sidStringPtr[0] !== undefined) {
          LocalFree(sidStringPtr[0]);
        }
      }
    } finally {
      CloseHandle(token);
    }
  } finally {
    CloseHandle(process);
  }
}

/** SID-compare half of server authentication; exported for Linux unit tests. */
export function assertWindowsServerSid(serverSid: string, currentUserSid: string): void {
  if (!FULL_SID_PATTERN.test(serverSid)) {
    failAuth('named-pipe server SID is not a canonical S-1-… string');
  }
  if (!FULL_SID_PATTERN.test(currentUserSid)) {
    failAuth('resolved current-user SID is not a canonical S-1-… SID string');
  }
  if (serverSid.toUpperCase() !== currentUserSid.toUpperCase()) {
    failAuth(
      `named-pipe server process SID ${serverSid} does not match the current user ${currentUserSid}`
    );
  }
}
