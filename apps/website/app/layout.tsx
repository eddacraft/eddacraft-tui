import React from 'react';
import type { Metadata } from 'next';
import { IBM_Plex_Sans, JetBrains_Mono } from 'next/font/google';
import { Analytics } from '@vercel/analytics/next';
import './globals.css';

const _jetbrainsMono = JetBrains_Mono({
  subsets: ['latin'],
  variable: '--font-jetbrains-mono',
});
const _ibmPlexSans = IBM_Plex_Sans({
  subsets: ['latin'],
  weight: ['400', '500', '600'],
  variable: '--font-ibm-plex-sans',
});

export const metadata: Metadata = {
  title: 'anvil — Trust the code your AI writes',
  description:
    'Independent, deterministic control for AI-assisted software engineering. Understand the change, apply your standards, and stop unsafe work before review.',
  generator: 'eddacraft',
  metadataBase: new URL('https://eddacraft.ai'),
  openGraph: {
    title: 'anvil — Trust the code your AI writes',
    description: 'Independent, deterministic control for AI-assisted software engineering.',
    siteName: 'anvil by eddacraft',
    locale: 'en_GB',
    type: 'website',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'anvil — Trust the code your AI writes',
    description: 'Independent, deterministic control for AI-assisted software engineering.',
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
    <html lang="en-GB">
      <body className={`${_jetbrainsMono.variable} ${_ibmPlexSans.variable} font-sans antialiased`}>
        {children}
        <Analytics />
      </body>
    </html>
  );
}
