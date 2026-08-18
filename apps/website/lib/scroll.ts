export function prefersReducedMotion(): boolean {
  return window.matchMedia('(prefers-reduced-motion: reduce)').matches;
}

export function scrollToWaitlist(): void {
  const waitlistSection = document.getElementById('waitlist');
  if (!waitlistSection) return;

  const reducedMotion = prefersReducedMotion();
  waitlistSection.scrollIntoView({ behavior: reducedMotion ? 'auto' : 'smooth' });

  const input = waitlistSection.querySelector('input');
  if (!(input instanceof HTMLInputElement)) return;

  window.setTimeout(() => input.focus(), reducedMotion ? 0 : 500);
}
