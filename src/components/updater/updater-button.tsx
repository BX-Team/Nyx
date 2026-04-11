import { listen } from '@tauri-apps/api/event';
import { CircleFadingArrowUp } from 'lucide-react';
import type React from 'react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { cancelUpdate } from '@/utils/ipc';
import UpdaterModal from './updater-modal';

interface Props {
  iconOnly?: boolean;
  latest?: {
    version: string;
    changelog: string;
  };
}

const UpdaterButton: React.FC<Props> = props => {
  const { t } = useTranslation();
  const { iconOnly, latest } = props;
  const [openModal, setOpenModal] = useState(false);
  const [updateStatus, setUpdateStatus] = useState<{
    downloading: boolean;
    progress: number;
    error?: string;
  }>({
    downloading: false,
    progress: 0,
  });

  useEffect(() => {
    const unlisten = listen<typeof updateStatus>('update-status', e => {
      setUpdateStatus(e.payload);
    });
    return (): void => {
      unlisten.then(f => f());
    };
  }, []);

  const handleCancelUpdate = async (): Promise<void> => {
    try {
      await cancelUpdate();
      setUpdateStatus({ downloading: false, progress: 0 });
    } catch (_e) {}
  };

  if (!latest) return null;

  return (
    <>
      {openModal && (
        <UpdaterModal
          version={latest.version}
          changelog={latest.changelog}
          updateStatus={updateStatus}
          onCancel={handleCancelUpdate}
          onClose={() => {
            setOpenModal(false);
          }}
        />
      )}
      {iconOnly ? (
        <Button
          size='icon'
          className='app-nodrag'
          variant='destructive'
          onClick={() => {
            setOpenModal(true);
          }}
        >
          <CircleFadingArrowUp />
        </Button>
      ) : (
        <Button
          size='default'
          className='app-nodrag w-full'
          variant='destructive'
          onClick={() => {
            setOpenModal(true);
          }}
        >
          <CircleFadingArrowUp />
          <span className='truncate'>{t('common.updateAvailable')}</span>
        </Button>
      )}
    </>
  );
};

export default UpdaterButton;
