import { ShieldCheck, TriangleAlert } from 'lucide-react';

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
  CommandShortcut,
} from '@/components/ui/command';

interface CommandSearchProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function CommandSearch({ open, onOpenChange }: CommandSearchProps) {
  const moveTo = (selector: string) => {
    onOpenChange(false);
    requestAnimationFrame(() => {
      const target = document.querySelector<HTMLElement>(selector);
      target?.scrollIntoView({ block: 'start', behavior: 'smooth' });
      target?.focus({ preventScroll: true });
    });
  };

  return (
    <CommandDialog
      className="dashboard-command"
      description="Jump to protection dashboard regions"
      onOpenChange={onOpenChange}
      open={open}
      title="Search dashboard"
    >
      <CommandInput aria-label="Search dashboard commands" placeholder="Search dashboard…" />
      <CommandList>
        <CommandEmpty>No dashboard regions found.</CommandEmpty>
        <CommandGroup heading="Protection">
          <CommandItem onSelect={() => moveTo('#protection-title')}>
            <ShieldCheck />
            Protection overview
            <CommandShortcut>↵</CommandShortcut>
          </CommandItem>
          <CommandItem onSelect={() => moveTo('#evidence-inspector-title')}>
            <TriangleAlert />
            Evidence inspector
          </CommandItem>
        </CommandGroup>
      </CommandList>
    </CommandDialog>
  );
}
