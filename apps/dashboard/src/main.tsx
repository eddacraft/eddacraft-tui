import { RouterProvider } from '@tanstack/react-router';
import { StrictMode, useMemo } from 'react';
import { createRoot } from 'react-dom/client';

import { createDashboardRouter } from './router';
import './styles.css';

export function DashboardApp() {
  const router = useMemo(() => createDashboardRouter(), []);

  return <RouterProvider router={router} />;
}

const rootElement = document.querySelector('#root');

if (rootElement) {
  createRoot(rootElement).render(
    <StrictMode>
      <DashboardApp />
    </StrictMode>
  );
}
