import { Switch as SwitchPrimitive } from 'radix-ui';
import type * as React from 'react';

import { cn } from '@/lib/utils';

function Switch({
  className,
  size = 'default',
  ...props
}: React.ComponentProps<typeof SwitchPrimitive.Root> & {
  size?: 'sm' | 'default';
}) {
  return (
    <SwitchPrimitive.Root
      data-slot='switch'
      data-size={size}
      className={cn(
        'peer group/switch inline-flex shrink-0 items-center rounded-full border transition-all outline-none disabled:cursor-not-allowed disabled:opacity-50',
        'data-[state=unchecked]:border-stroke data-[state=unchecked]:bg-card/50 data-[state=unchecked]:backdrop-blur-xl',
        'data-[state=checked]:border-stroke-power-on data-[state=checked]:bg-gradient-to-r data-[state=checked]:from-gradient-start-power-on data-[state=checked]:to-gradient-end-power-on',
        'focus-visible:ring-2 focus-visible:ring-stroke',
        'data-[size=default]:h-5 data-[size=default]:w-9 data-[size=sm]:h-4 data-[size=sm]:w-7',
        className,
      )}
      {...props}
    >
      <SwitchPrimitive.Thumb
        data-slot='switch-thumb'
        className={cn(
          'pointer-events-none block rounded-full ring-0 transition-transform',
          'data-[state=unchecked]:bg-muted-foreground/60 data-[state=checked]:bg-white',
          'group-data-[size=default]/switch:size-3.5 group-data-[size=sm]/switch:size-2.5',
          'data-[state=checked]:translate-x-[calc(100%+4px)] data-[state=unchecked]:translate-x-0.5',
        )}
      />
    </SwitchPrimitive.Root>
  );
}

export { Switch };
