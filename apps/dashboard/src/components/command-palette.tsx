import { useNavigate } from '@tanstack/react-router';
import { ShieldCheck } from 'lucide-react';

import {
  CommandDialog,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from '@/components/ui/command';
import { createCommandEntries } from '@/hooks/use-command-search';
import { dashboardModuleRegistry } from '@/modules/registry';

const commandEntries = createCommandEntries(dashboardModuleRegistry);

export function CommandPalette({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  const navigate = useNavigate();
  const select = (entry: (typeof commandEntries)[number]) => {
    onOpenChange(false);
    void navigate({ to: entry.to, search: (previous) => ({ ...previous, ...entry.search }) });
  };
  return (
    <CommandDialog
      description="Jump to registered dashboard modules and resources"
      onOpenChange={onOpenChange}
      open={open}
      title="Search dashboard"
    >
      <CommandInput aria-label="Search dashboard commands" placeholder="Search dashboard…" />
      <CommandList>
        <CommandEmpty>No registered dashboard resources found.</CommandEmpty>
        {[
          'Modules',
          ...dashboardModuleRegistry.manifests.map((manifest) => manifest.navigation.label),
        ].map((group) => (
          <CommandGroup heading={group} key={group}>
            {commandEntries
              .filter((entry) => entry.group === group)
              .map((entry) => (
                <CommandItem
                  key={entry.id}
                  onSelect={() => select(entry)}
                  value={`${entry.group} ${entry.label}`}
                >
                  <ShieldCheck aria-hidden="true" /> {entry.label}
                </CommandItem>
              ))}
          </CommandGroup>
        ))}
      </CommandList>
    </CommandDialog>
  );
}
