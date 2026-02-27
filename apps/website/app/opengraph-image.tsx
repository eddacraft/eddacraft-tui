import { ImageResponse } from 'next/og';

export const runtime = 'edge';

export const alt = 'Anvil — AI Governance for Developers';
export const size = {
  width: 1200,
  height: 630,
};
export const contentType = 'image/png';

export default async function Image() {
  return new ImageResponse(
    <div
      style={{
        background: '#0D0D0F',
        width: '100%',
        height: '100%',
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        justifyContent: 'center',
        fontFamily: 'monospace',
      }}
    >
      {/* Anvil Brandmark */}
      <svg width="120" height="120" viewBox="0 0 554 554" fill="none">
        <path d="M553.604 0H388.719V74.1044H553.604V0Z" fill="#CC5500" />
        <path d="M553.604 72.3608H478.423V480.371H553.604V72.3608Z" fill="#CC5500" />
        <path d="M553.604 479.499H388.719V553.604H553.604V479.499Z" fill="#CC5500" />
        <path d="M387.522 166.081H166.081V387.523H387.522V166.081Z" fill="#CC5500" />
        <path d="M0 0H164.885V74.1044H0V0Z" fill="#CC5500" />
        <path d="M0 72.3608H75.1807V480.371H0V72.3608Z" fill="#CC5500" />
        <path d="M0 479.499H164.885V553.604H0V479.499Z" fill="#CC5500" />
      </svg>

      {/* Product Name */}
      <div
        style={{
          fontSize: 32,
          fontWeight: 400,
          color: '#CC5500',
          letterSpacing: '0.3em',
          textTransform: 'uppercase',
          marginTop: 32,
        }}
      >
        ANVIL
      </div>

      {/* Tagline */}
      <div
        style={{
          fontSize: 48,
          fontWeight: 600,
          color: '#EBEBEB',
          textTransform: 'uppercase',
          marginTop: 40,
          textAlign: 'center',
          maxWidth: 900,
          lineHeight: 1.2,
        }}
      >
        FORCE PROBABILISTIC TOOLS TO
      </div>
      <div
        style={{
          fontSize: 48,
          fontWeight: 600,
          color: '#CC5500',
          textTransform: 'uppercase',
          textAlign: 'center',
          maxWidth: 900,
          lineHeight: 1.2,
        }}
      >
        RESPECT DETERMINISTIC RULES.
      </div>

      {/* Subtext */}
      <div
        style={{
          fontSize: 20,
          color: '#85858A',
          marginTop: 40,
        }}
      >
        AI Governance for Developers
      </div>

      {/* Footer */}
      <div
        style={{
          position: 'absolute',
          bottom: 40,
          display: 'flex',
          alignItems: 'center',
          gap: 12,
          color: '#85858A',
          fontSize: 16,
        }}
      >
        <span>eddacraft.ai</span>
        <span style={{ color: '#2A2A2E' }}>|</span>
        <span>@eddacraft</span>
      </div>
    </div>,
    {
      ...size,
    }
  );
}
