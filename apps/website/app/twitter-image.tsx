import { ImageResponse } from 'next/og';
import { SocialCard } from './social-card';

export const runtime = 'edge';
export const alt = 'anvil — Trust the code your AI writes';
export const size = { width: 1200, height: 630 };
export const contentType = 'image/png';

export default function Image() {
  return new ImageResponse(<SocialCard />, size);
}
