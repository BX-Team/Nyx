import { cva, type VariantProps } from 'class-variance-authority';
import { Tabs as TabsPrimitive } from 'radix-ui';
import type * as React from 'react';

import { cn } from '@/lib/utils';

function Tabs({ className, orientation = 'horizontal', ...props }: React.ComponentProps<typeof TabsPrimitive.Root>) {
  return (
    <TabsPrimitive.Root
      data-slot='tabs'
      data-orientation={orientation}
      orientation={orientation}
      className={cn('group/tabs flex gap-2 data-[orientation=horizontal]:flex-col', className)}
      {...props}
    />
  );
}

const tabsListVariants = cva(
  'rounded-lg p-[3px] group-data-[orientation=horizontal]/tabs:h-8 data-[variant=line]:rounded-none group/tabs-list text-muted-foreground inline-flex w-fit items-center justify-center group-data-[orientation=vertical]/tabs:h-fit group-data-[orientation=vertical]/tabs:flex-col',
  {
    variants: {
      variant: {
        default: 'border border-stroke bg-card/50 backdrop-blur-xl',
        line: 'gap-1 bg-transparent',
      },
    },
    defaultVariants: {
      variant: 'default',
    },
  },
);

function TabsList({
  className,
  variant = 'default',
  ...props
}: React.ComponentProps<typeof TabsPrimitive.List> & VariantProps<typeof tabsListVariants>) {
  return (
    <TabsPrimitive.List
      data-slot='tabs-list'
      data-variant={variant}
      className={cn(tabsListVariants({ variant }), className)}
      {...props}
    />
  );
}

function TabsTrigger({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Trigger>) {
  return (
    <TabsPrimitive.Trigger
      data-slot='tabs-trigger'
      className={cn(
        'relative inline-flex h-[calc(100%-1px)] items-center justify-center gap-1 rounded-md px-2 py-0.5 text-xs font-medium whitespace-nowrap transition-all duration-200',
        'text-muted-foreground hover:text-foreground',
        'border border-transparent',
        'group-data-[orientation=vertical]/tabs:w-full group-data-[orientation=vertical]/tabs:justify-start',
        'disabled:pointer-events-none disabled:opacity-50',
        "[&_svg]:pointer-events-none [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
        'focus-visible:border-ring focus-visible:ring-ring/50 focus-visible:outline-ring focus-visible:ring-[3px] focus-visible:outline-1',
        'group-data-[variant=default]/tabs-list:data-[state=active]:bg-gradient-to-br group-data-[variant=default]/tabs-list:data-[state=active]:from-gradient-start-power-on/15 group-data-[variant=default]/tabs-list:data-[state=active]:to-gradient-end-power-on/15',
        'group-data-[variant=default]/tabs-list:data-[state=active]:border-stroke-power-on/50',
        'group-data-[variant=default]/tabs-list:data-[state=active]:text-foreground',
        'group-data-[variant=default]/tabs-list:data-[state=active]:shadow-sm',
        'group-data-[variant=line]/tabs-list:bg-transparent group-data-[variant=line]/tabs-list:data-[state=active]:bg-transparent',
        'after:absolute after:opacity-0 after:transition-opacity after:bg-gradient-end-power-on',
        'group-data-[orientation=horizontal]/tabs:after:inset-x-0 group-data-[orientation=horizontal]/tabs:after:bottom-[-5px] group-data-[orientation=horizontal]/tabs:after:h-0.5 group-data-[orientation=horizontal]/tabs:after:rounded-full',
        'group-data-[orientation=vertical]/tabs:after:inset-y-0 group-data-[orientation=vertical]/tabs:after:-right-1 group-data-[orientation=vertical]/tabs:after:w-0.5 group-data-[orientation=vertical]/tabs:after:rounded-full',
        'group-data-[variant=line]/tabs-list:data-[state=active]:after:opacity-100 group-data-[variant=line]/tabs-list:data-[state=active]:text-foreground',
        className,
      )}
      {...props}
    />
  );
}

function TabsContent({ className, ...props }: React.ComponentProps<typeof TabsPrimitive.Content>) {
  return <TabsPrimitive.Content data-slot='tabs-content' className={cn('flex-1 outline-none', className)} {...props} />;
}

export { Tabs, TabsContent, TabsList, TabsTrigger, tabsListVariants };
