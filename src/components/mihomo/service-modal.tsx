import { t } from 'i18next';
import type React from 'react';
import { useCallback, useEffect, useState } from 'react';
import { toast } from 'sonner';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Dialog, DialogClose, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Separator } from '@/components/ui/separator';
import { Spinner } from '@/components/ui/spinner';
import { serviceStatus, testServiceConnection } from '@/utils/ipc';

interface Props {
  onChange: (open: boolean) => void;
  onInit: () => Promise<void>;
  onInstall: () => Promise<void>;
  onUninstall: () => Promise<void>;
  onStart: () => Promise<void>;
  onRestart: () => Promise<void>;
  onStop: () => Promise<void>;
}

type ServiceStatusType = 'running' | 'stopped' | 'not-installed' | 'unknown' | 'need-init';
type ConnectionStatusType = 'connected' | 'disconnected' | 'checking' | 'unknown';

const ServiceModal: React.FC<Props> = props => {
  const { onChange, onInit, onInstall, onUninstall, onStart, onStop, onRestart } = props;
  const [loading, setLoading] = useState(false);
  const [status, setStatus] = useState<ServiceStatusType | null>(null);
  const [connectionStatus, setConnectionStatus] = useState<ConnectionStatusType>('checking');

  const checkServiceConnection = useCallback(async (): Promise<void> => {
    if (status === 'running') {
      try {
        setConnectionStatus('checking');
        const connected = await testServiceConnection();
        setConnectionStatus(connected ? 'connected' : 'disconnected');
      } catch {
        setConnectionStatus('disconnected');
      }
    } else {
      setConnectionStatus('disconnected');
    }
  }, [status]);

  useEffect(() => {
    const checkStatus = async (): Promise<void> => {
      try {
        const result = await serviceStatus();
        setStatus(result);
      } catch {
        setStatus('not-installed');
      }
    };
    checkStatus();
  }, []);

  useEffect(() => {
    checkServiceConnection();
  }, [checkServiceConnection]);

  const handleAction = async (action: () => Promise<void>, isStartAction = false): Promise<void> => {
    setLoading(true);
    try {
      await action();

      await new Promise(resolve => setTimeout(resolve, 500));

      let result = await serviceStatus();

      if (isStartAction) {
        let retries = 5;
        while (retries > 0 && result === 'stopped') {
          await new Promise(resolve => setTimeout(resolve, 1000));
          result = await serviceStatus();
          retries--;
        }
      }

      setStatus(result);
      await checkServiceConnection();
    } catch (e) {
      const errorMsg = String(e);
      if (errorMsg.includes(t('common.cancel', { lng: 'zh-CN' })) || errorMsg.includes('UserCancelledError')) {
        const result = await serviceStatus();
        setStatus(result);
        await checkServiceConnection();
        return;
      }
      toast.error(`${e}`);
    } finally {
      setLoading(false);
    }
  };

  const getStatusText = (): string => {
    if (status === null) return t('mihomo.serviceModal.checking');
    switch (status) {
      case 'running':
        return t('mihomo.serviceModal.running');
      case 'stopped':
        return t('mihomo.serviceModal.stopped');
      case 'not-installed':
        return t('mihomo.serviceModal.notInstalled');
      case 'need-init':
        return t('mihomo.serviceModal.needInit');
      default:
        return t('mihomo.serviceModal.unknownStatus');
    }
  };

  const getConnectionStatusText = (): string => {
    switch (connectionStatus) {
      case 'connected':
        return t('mihomo.serviceModal.connected');
      case 'disconnected':
        return t('mihomo.serviceModal.disconnected');
      case 'checking':
        return t('mihomo.serviceModal.connectionChecking');
      default:
        return t('mihomo.serviceModal.unknown');
    }
  };

  const getServiceStatusBadgeClass = (): string => {
    if (status === null) return 'bg-muted-foreground text-white animate-pulse';
    switch (status) {
      case 'running':
        return 'bg-success text-white';
      case 'stopped':
        return 'bg-warning text-white';
      case 'not-installed':
        return 'bg-destructive text-white';
      case 'need-init':
        return 'bg-warning text-white';
      default:
        return 'bg-muted-foreground text-white';
    }
  };

  const getConnectionBadgeClass = (): string => {
    switch (connectionStatus) {
      case 'checking':
        return 'bg-muted-foreground text-white animate-pulse';
      case 'connected':
        return 'bg-success text-white';
      case 'disconnected':
        return 'bg-destructive text-white';
      default:
        return 'bg-muted-foreground text-white';
    }
  };

  return (
    <Dialog open={true} onOpenChange={onChange}>
      <DialogContent className='w-[450px] max-w-[calc(100%-2rem)] max-h-[70vh] flex flex-col' showCloseButton={false}>
        <DialogHeader className='flex flex-col gap-1'>
          <DialogTitle>{t('mihomo.serviceModal.title')}</DialogTitle>
        </DialogHeader>
        <div className='flex-1 min-h-0 overflow-y-auto'>
          <div className='space-y-4'>
            <Card className='border-none bg-linear-to-br from-muted/30 to-muted/50 py-0 gap-0'>
              <CardContent className='py-4'>
                <div className='flex items-center justify-between mb-3'>
                  <div className='flex items-center gap-2'>
                    <span className='text-sm font-medium'>{t('mihomo.serviceModal.serviceStatus')}</span>
                  </div>
                  {status === null ? (
                    <Badge className={`text-xs gap-2 ${getServiceStatusBadgeClass()}`}>
                      <Spinner className='size-3' />
                      {t('mihomo.serviceModal.checkingEllipsis')}
                    </Badge>
                  ) : (
                    <Badge className={`text-xs ${getServiceStatusBadgeClass()}`}>{getStatusText()}</Badge>
                  )}
                </div>

                <div className='flex items-center justify-between'>
                  <div className='flex items-center gap-2'>
                    <span className='text-sm font-medium'>{t('mihomo.serviceModal.connectionStatus')}</span>
                  </div>
                  {connectionStatus === 'checking' ? (
                    <Badge className={`text-xs gap-2 ${getConnectionBadgeClass()}`}>
                      <Spinner className='size-3' />
                      {t('mihomo.serviceModal.connectionCheckingEllipsis')}
                    </Badge>
                  ) : (
                    <Badge className={`text-xs ${getConnectionBadgeClass()}`}>{getConnectionStatusText()}</Badge>
                  )}
                </div>
              </CardContent>
            </Card>

            <Separator />

            <div className='text-xs text-muted-foreground space-y-2'>
              <div className='flex items-start gap-2'>
                <span>{t('mihomo.serviceModal.description1')}</span>
              </div>
              <div className='flex items-start gap-2'>
                <span>{t('mihomo.serviceModal.description2')}</span>
              </div>
              <div className='flex items-start gap-2'>
                <span>{t('mihomo.serviceModal.description3')}</span>
              </div>
              <div className='flex items-start gap-2'>
                <span>{t('mihomo.serviceModal.description4')}</span>
              </div>
            </div>
          </div>
        </div>
        <DialogFooter className='flex-row flex-wrap justify-end gap-2'>
          <DialogClose asChild>
            <Button size='sm' variant='ghost' disabled={loading} className='sm:mr-auto'>
              {t('common.close')}
            </Button>
          </DialogClose>

          {status === 'unknown' ? null : status === 'not-installed' ? (
            <Button size='sm' className='shadow-sm' onClick={() => handleAction(onInstall)} disabled={loading}>
              {loading && <Spinner className='mr-2 size-4' />}
              {t('mihomo.serviceModal.installService')}
            </Button>
          ) : (
            <>
              <Button
                size='sm'
                variant='outline'
                className='border-primary/40 text-primary hover:bg-primary/10'
                onClick={() => handleAction(onInit)}
                disabled={loading}
              >
                {loading && <Spinner className='mr-2 size-4' />}
                {t('mihomo.serviceModal.init')}
              </Button>
              <Button
                size='sm'
                variant='outline'
                className='border-primary/40 text-primary hover:bg-primary/10'
                onClick={() => handleAction(onRestart)}
                disabled={loading}
              >
                {loading && <Spinner className='mr-2 size-4' />}
                {t('mihomo.serviceModal.restart')}
              </Button>
              {status === 'running' || status === 'need-init' ? (
                <Button
                  size='sm'
                  variant='outline'
                  className='border-warning text-warning hover:bg-warning/10'
                  onClick={() => handleAction(onStop)}
                  disabled={loading}
                >
                  {loading && <Spinner className='mr-2 size-4' />}
                  {t('mihomo.serviceModal.stop')}
                </Button>
              ) : (
                <Button
                  size='sm'
                  className='bg-success text-white hover:bg-success/90 shadow-sm'
                  onClick={() => handleAction(onStart, true)}
                  disabled={loading}
                >
                  {loading && <Spinner className='mr-2 size-4' />}
                  {t('mihomo.serviceModal.start')}
                </Button>
              )}
              <Button size='sm' variant='destructive' onClick={() => handleAction(onUninstall)} disabled={loading}>
                {loading && <Spinner className='mr-2 size-4' />}
                {t('mihomo.serviceModal.uninstall')}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default ServiceModal;
