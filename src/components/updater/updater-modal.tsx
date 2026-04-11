import { Download, X } from 'lucide-react';
import type React from 'react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import ReactMarkdown from 'react-markdown';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { Spinner } from '@/components/ui/spinner';
import { downloadAndInstallUpdate } from '@/utils/ipc';

interface Props {
  version: string;
  changelog: string;
  updateStatus?: {
    downloading: boolean;
    progress: number;
    error?: string;
  };
  onCancel?: () => void;
  onClose: () => void;
}

const UpdaterModal: React.FC<Props> = props => {
  const { t } = useTranslation();
  const { version, changelog, updateStatus, onCancel, onClose } = props;
  const [downloading, setDownloading] = useState(false);
  const onUpdate = async (): Promise<void> => {
    try {
      setDownloading(true);
      await downloadAndInstallUpdate(version);
    } catch (e) {
      toast.error(`${e}`);
      setDownloading(false);
    }
  };
  const handleCancel = (): void => {
    if (updateStatus?.downloading && onCancel) {
      setDownloading(false);
      onCancel();
    } else {
      onClose();
    }
  };

  const isDownloading = updateStatus?.downloading || downloading;

  return (
    <Dialog
      open={true}
      onOpenChange={open => {
        if (!open && !isDownloading) onClose();
      }}
    >
      <DialogContent
        className='h-[calc(100%-111px)] w-[calc(100%-100px)] max-w-none sm:max-w-none flex flex-col'
        showCloseButton={false}
      >
        <DialogHeader className='app-drag flex-row items-center justify-between'>
          <DialogTitle className='flex items-center gap-2'>
            <Download className='text-lg' />
            {t('updater.versionReady', { version })}
          </DialogTitle>
          {!isDownloading && (
            <Button
              size='sm'
              variant='outline'
              className='app-nodrag'
              onClick={() => {
                open(`https://github.com/coolcoala/koala-clash/releases/tag/${version}`);
              }}
            >
              {t('updater.goToDownload')}
            </Button>
          )}
        </DialogHeader>
        <div className='flex-1 min-h-0 overflow-y-auto'>
          {updateStatus?.downloading && (
            <div className='space-y-3 mb-4'>
              <div className='flex items-center justify-between'>
                <span className='text-sm text-muted-foreground'>{t('updater.downloadProgress')}</span>
                <span className='text-sm font-medium'>{updateStatus.progress}%</span>
              </div>
              <Progress value={updateStatus.progress} />
              {updateStatus.error && <div className='text-destructive text-sm'>{updateStatus.error}</div>}
            </div>
          )}
          {!updateStatus?.downloading && (
            <div className='markdown-body'>
              <ReactMarkdown
                components={{
                  a: ({ ...props }) => <a target='_blank' className='text-primary' {...props} />,
                  code: ({ className, children, ...props }) => (
                    <code
                      className={['rounded bg-muted px-1.5 py-0.5 font-mono text-xs', className]
                        .filter(Boolean)
                        .join(' ')}
                      {...props}
                    >
                      {children}
                    </code>
                  ),
                  pre: ({ children, ...props }) => (
                    <pre
                      className='rounded bg-muted p-3 overflow-x-auto [&>code]:bg-transparent [&>code]:p-0 [&>code]:rounded-none'
                      {...props}
                    >
                      {children}
                    </pre>
                  ),
                  h3: ({ ...props }) => <h3 className='text-lg font-bold' {...props} />,
                  li: ({ children }) => <li className='list-disc list-inside'>{children}</li>,
                }}
              >
                {changelog}
              </ReactMarkdown>
            </div>
          )}
        </div>
        <DialogFooter className='gap-2'>
          <Button size='sm' variant='ghost' onClick={handleCancel}>
            {updateStatus?.downloading ? <X className='mr-2' /> : null}
            {updateStatus?.downloading ? t('updater.cancelDownload') : t('common.cancel')}
          </Button>
          {!updateStatus?.downloading && (
            <Button size='sm' onClick={onUpdate} disabled={downloading}>
              {downloading ? <Spinner className='mr-2 size-4' /> : <Download className='mr-2' />}
              {t('updater.updateNow')}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
};

export default UpdaterModal;
