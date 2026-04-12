import { Globe, Route } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useSidebar } from '@/components/ui/sidebar';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useAppConfig } from '@/hooks/use-app-config';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { useGroups } from '@/hooks/use-groups';
import { cn } from '@/lib/utils';
import { mihomoChangeProxy, mihomoCloseAllConnections } from '@/utils/ipc';

const SIDEBAR_ANIMATION_MS = 200;

const OutboundModeSwitcher: React.FC = () => {
  const { t } = useTranslation();
  const { state, isMobile } = useSidebar();
  const collapsed = state === 'collapsed';
  const { controledMihomoConfig, patchControledMihomoConfig } = useControledMihomoConfig();
  const { groups, mutate: mutateGroups } = useGroups();
  const { appConfig } = useAppConfig();
  const { autoCloseConnection = true } = appConfig || {};
  const { mode } = controledMihomoConfig || {};

  const [iconOnly, setIconOnly] = useState(collapsed);
  const [fading, setFading] = useState(false);

  useEffect(() => {
    if (collapsed) {
      setFading(true);
      const timer = setTimeout(() => {
        setIconOnly(true);
        setFading(false);
      }, 50);
      return () => clearTimeout(timer);
    } else {
      setFading(true);
      const timer = setTimeout(() => {
        setIconOnly(false);
        setFading(false);
      }, SIDEBAR_ANIMATION_MS);
      return () => clearTimeout(timer);
    }
  }, [collapsed]);

  const onChangeMode = async (newMode: OutboundMode): Promise<void> => {
    await patchControledMihomoConfig({ mode: newMode });
    if (newMode === 'global') {
      const firstSelector = groups?.find(g => g.type === 'Selector');
      const currentProxy = firstSelector?.now;
      if (currentProxy && currentProxy !== 'DIRECT') {
        await mihomoChangeProxy('GLOBAL', currentProxy);
      }
    }
    if (autoCloseConnection) {
      await mihomoCloseAllConnections();
    }
    mutateGroups();
  };

  if (!mode) return null;

  const modes = [
    { value: 'rule' as const, icon: Route, label: t('sider.rules') },
    { value: 'global' as const, icon: Globe, label: t('common.global') },
  ];

  return (
    <div
      className={cn(
        'flex items-center rounded-lg border border-stroke bg-card/50 backdrop-blur-xl p-[3px] transition-opacity duration-150',
        iconOnly ? 'flex-col gap-1' : 'w-full gap-1',
        fading ? 'opacity-0' : 'opacity-100',
      )}
    >
      {modes.map(({ value, icon: Icon, label }) => (
        <Tooltip key={value}>
          <TooltipTrigger asChild>
            <button
              type='button'
              onClick={() => onChangeMode(value)}
              className={cn(
                'flex items-center justify-center rounded-md font-medium transition-colors',
                iconOnly ? 'size-8' : 'h-7 flex-1 gap-1.5 px-2 text-xs',
                mode === value
                  ? 'bg-gradient-to-br from-gradient-start-power-on/15 to-gradient-end-power-on/15 border border-stroke-power-on/50 text-foreground shadow-sm'
                  : 'border border-transparent text-muted-foreground hover:text-foreground',
              )}
            >
              <Icon className={cn('shrink-0', iconOnly ? 'size-4' : 'size-3.5')} />
              {!iconOnly && <span>{label}</span>}
            </button>
          </TooltipTrigger>
          <TooltipContent side='right' hidden={!iconOnly || isMobile}>
            {label}
          </TooltipContent>
        </Tooltip>
      ))}
    </div>
  );
};

export default OutboundModeSwitcher;
