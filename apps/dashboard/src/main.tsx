import { RouterProvider } from '@tanstack/react-router';
import { StrictMode, useMemo } from 'react';
import { createRoot } from 'react-dom/client';

import { DashboardQueryProvider } from './api/query-client';
import { createDashboardRouter } from './router';
import './styles.css';

export function DashboardApp() {
  const router = useMemo(() => createDashboardRouter(), []);

  return (
    <DashboardQueryProvider>
      <RouterProvider router={router} />
    </DashboardQueryProvider>
  );
}

const rootElement = document.querySelector('#root');

if (rootElement) {
  createRoot(rootElement).render(
    <StrictMode>
      <DashboardApp />
    </StrictMode>
  );
}
