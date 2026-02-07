import React from 'react';
import type { Metadata } from 'next';
import { JetBrains_Mono, Inter } from 'next/font/google';
import { Analytics } from '@vercel/analytics/next';
import './globals.css';

const _jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-mono',
});
const _inter = Inter({
  subsets: ['latin'],
  variable: '--font-sans',
});

export const metadata: Metadata = {
  title: 'Anvil — AI Governance for Developers',
  description:
    'Force probabilistic tools to respect deterministic rules. Anvil enforces policy at generation time — not at review.',
  generator: 'eddacraft',
  metadataBase: new URL('https://anvil.eddacraft.com'),
  openGraph: {
    title: 'Anvil — AI Governance for Developers',
    description:
      'Force probabilistic tools to respect deterministic rules. Anvil enforces policy at generation time — not at review.',
    siteName: 'Anvil by EddaCraft',
    locale: 'en_GB',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'Anvil — AI Governance for Developers',
    description:
      'Force probabilistic tools to respect deterministic rules. Anvil enforces policy at generation time — not at review.',
    creator: '@eddacraft',
  },
  icons: {
    icon: [
      { url: '/icon.svg', type: 'image/svg+xml' },
      {
        url: '/icon-light-32x32.png',
        sizes: '32x32',
        type: 'image/png',
        media: '(prefers-color-scheme: light)',
      },
      {
        url: '/icon-dark-32x32.png',
        sizes: '32x32',
        type: 'image/png',
        media: '(prefers-color-scheme: dark)',
      },
    ],
    shortcut: '/favicon.ico',
    apple: '/apple-icon.png',
  },
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en">
      <body className={`${_jetbrainsMono.variable} ${_inter.variable} font-sans antialiased`}>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
