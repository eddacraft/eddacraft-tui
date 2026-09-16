export interface WaitlistSubmitResult {
  success: boolean;
  error?: string;
  warning?: string;
  emailSent?: boolean;
  isNewSignup?: boolean;
}

export interface WaitlistResponseLine {
  text: string;
  colorClass: string;
  delay: number;
}

const EMAIL_REGEX = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;

export function isValidEmail(email: string): boolean {
  return EMAIL_REGEX.test(email);
}

export function getWaitlistEndpoint(): string {
  const apiBase = (process.env.NEXT_PUBLIC_API_URL ?? 'https://api.eddacraft.ai').replace(
    /\/+$/,
    ''
  );
  return `${apiBase}/api/v1/waitlist`;
}

// Shared by the hero form and the footer CLI form so both post the same payload
// to the same endpoint and surface the same warnings. The API-side per-email
// throttle counts every submission, so callers should guard against double
// submits while a request is in flight.
export async function submitToWaitlist(email: string): Promise<WaitlistSubmitResult> {
  try {
    const response = await fetch(getWaitlistEndpoint(), {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ email }),
    });
    const data = (await response.json()) as {
      error?: string;
      warning?: string;
      emailSent?: boolean;
      isNewSignup?: boolean;
    };
    if (!response.ok) {
      return {
        success: false,
        error: data.error || 'Failed to join waitlist',
        warning: data.warning,
        emailSent: data.emailSent,
        isNewSignup: data.isNewSignup,
      };
    }

    return {
      success: true,
      warning: data.warning,
      emailSent: data.emailSent,
      isNewSignup: data.isNewSignup,
    };
  } catch {
    return { success: false, error: 'Network error. Please try again.' };
  }
}

export function buildResponseLines(
  userEmail: string,
  submitWarning: string | null,
  emailFailed: boolean
): WaitlistResponseLine[] {
  const lines: WaitlistResponseLine[] = [
    { text: 'Verifying...', colorClass: 'text-ghost-grey', delay: 600 },
    { text: '[ OK ] Access request received', colorClass: 'text-edda', delay: 400 },
    {
      text:
        submitWarning && submitWarning.includes('WARN')
          ? `Access is queued for ${userEmail}`
          : `We will be in touch at ${userEmail}`,
      colorClass: 'text-ghost-grey',
      delay: 0,
    },
  ];
  if (emailFailed) {
    lines.push({
      text: '[ WARN ] Confirmation email could not be sent — you are still on the list',
      colorClass: 'text-dull-amber',
      delay: 300,
    });
  }
  return lines;
}
