import { Skeleton } from '@/components/ui/skeleton';

export interface LoadingSkeletonProps {
  label: string;
  rows?: number;
}

export function LoadingSkeleton({ label, rows = 3 }: LoadingSkeletonProps) {
  return (
    <div aria-label={label} aria-live="polite" className="flex flex-col gap-2" role="status">
      <span className="sr-only">{label}</span>
      {Array.from({ length: rows }, (_, index) => (
        <Skeleton className="h-6 w-full" key={index} />
      ))}
    </div>
  );
}
