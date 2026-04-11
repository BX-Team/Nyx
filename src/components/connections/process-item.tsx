import { ChevronRight } from 'lucide-react';
import type React from 'react';
import { memo, useMemo } from 'react';
import { useTranslation } from 'react-i18next';
import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
import { Badge } from '@/components/ui/badge';
import { calcTraffic } from '@/utils/calc';

export interface ProcessGroup {
  processPath: string;
  processName: string;
  displayName?: string;
  iconUrl: string;
  activeCount: number;
  closedCount: number;
  totalUpload: number;
  totalDownload: number;
  totalUploadSpeed: number;
  totalDownloadSpeed: number;
}

interface Props {
  process: ProcessGroup;
  displayIcon: boolean;
  onClick: (processPath: string) => void;
}

const ProcessItemComponent: React.FC<Props> = ({ process, displayIcon, onClick }) => {
  const { t } = useTranslation();

  const uploadTraffic = useMemo(() => calcTraffic(process.totalUpload), [process.totalUpload]);
  const downloadTraffic = useMemo(() => calcTraffic(process.totalDownload), [process.totalDownload]);

  const uploadSpeed = useMemo(
    () => (process.totalUploadSpeed ? calcTraffic(process.totalUploadSpeed) : null),
    [process.totalUploadSpeed],
  );
  const downloadSpeed = useMemo(
    () => (process.totalDownloadSpeed ? calcTraffic(process.totalDownloadSpeed) : null),
    [process.totalDownloadSpeed],
  );

  const hasSpeed = useMemo(
    () => Boolean(process.totalUploadSpeed || process.totalDownloadSpeed),
    [process.totalUploadSpeed, process.totalDownloadSpeed],
  );

  const name = process.displayName || process.processName || t('pages.connections.unknownProcess');
  const hasActive = process.activeCount > 0;

  return (
    <div className='px-2 pb-2' style={{ height: 72 }}>
      <div
        className={`
          w-full h-full flex items-center cursor-pointer rounded-xl border
          transition-all duration-200 ease-out
          ${
            hasActive
              ? 'border-stroke-power-on/30 bg-linear-to-r from-gradient-start-power-on/[0.06] to-card/40 hover:border-stroke-power-on/50 shadow-sm'
              : 'border-border bg-card/40 hover:bg-accent/50'
          }
        `}
        onClick={() => onClick(process.processPath)}
      >
        <div className='w-full flex items-center'>
          {displayIcon && (
            <div className='pl-3'>
              <Avatar className='size-12 rounded-lg'>
                <AvatarImage src={process.iconUrl} />
                <AvatarFallback className='rounded-lg text-xs font-semibold text-muted-foreground'>
                  {name.slice(0, 2).toUpperCase()}
                </AvatarFallback>
              </Avatar>
            </div>
          )}
          <div className={`flex-1 flex flex-col truncate ${displayIcon ? 'pl-3' : 'pl-4'} pr-1`}>
            <div className='flex items-center gap-2'>
              <span className='text-sm font-medium truncate'>{name}</span>
              <div className='flex items-center gap-1 shrink-0'>
                {hasActive && (
                  <Badge className='min-w-5 h-5 justify-center px-1.5 leading-none text-[11px] bg-gradient-end-power-on text-white border-0'>
                    {process.activeCount}
                  </Badge>
                )}
                {process.closedCount > 0 && (
                  <Badge
                    variant='outline'
                    className='min-w-5 h-5 justify-center px-1.5 leading-none text-[11px] text-muted-foreground'
                  >
                    {process.closedCount}
                  </Badge>
                )}
              </div>
            </div>
            <div className='flex items-center gap-1.5 mt-1'>
              <span className='text-xs text-muted-foreground'>
                {'\u2191'} {uploadTraffic} {'\u2193'} {downloadTraffic}
              </span>
              {hasSpeed && (
                <>
                  <span className='text-xs text-muted-foreground/40'>|</span>
                  <span className={`text-xs ${hasActive ? 'text-gradient-end-power-on' : 'text-muted-foreground'}`}>
                    {'\u2191'} {uploadSpeed || '0 B'}/s {'\u2193'} {downloadSpeed || '0 B'}/s
                  </span>
                </>
              )}
            </div>
          </div>
          <div className='pr-3 shrink-0'>
            <ChevronRight className='text-muted-foreground/50 size-4' />
          </div>
        </div>
      </div>
    </div>
  );
};

const ProcessItem = memo(ProcessItemComponent, (prevProps, nextProps) => {
  const prev = prevProps.process;
  const next = nextProps.process;
  return (
    prev.processPath === next.processPath &&
    prev.activeCount === next.activeCount &&
    prev.closedCount === next.closedCount &&
    prev.totalUpload === next.totalUpload &&
    prev.totalDownload === next.totalDownload &&
    prev.totalUploadSpeed === next.totalUploadSpeed &&
    prev.totalDownloadSpeed === next.totalDownloadSpeed &&
    prev.iconUrl === next.iconUrl &&
    prev.displayName === next.displayName &&
    prevProps.displayIcon === nextProps.displayIcon
  );
});

export default ProcessItem;
