import { listen } from '@tauri-apps/api/event';
import { MessageCircleQuestionMark, Settings } from 'lucide-react';
import { useEffect, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import useSWR from 'swr';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { version } from '@/utils/init';
import {
  cancelUpdate,
  checkUpdate,
  debugInfo,
  mihomoInstalledVersion,
  mihomoVersion,
  quitApp,
  quitWithoutCore,
  resetAppConfig,
} from '@/utils/ipc';
import ConfirmModal from '../base/base-confirm';
import SettingCard from '../base/base-setting-card';
import SettingItem from '../base/base-setting-item';
import UpdaterModal from '../updater/updater-modal';

const EASTER_EGG_TAP_COUNT = 7;

interface ActionsProps {
  showHiddenSettings: boolean;
  onUnlockHiddenSettings: () => void;
}

const Actions: React.FC<ActionsProps> = props => {
  const { showHiddenSettings, onUnlockHiddenSettings } = props;
  const { t } = useTranslation();
  const navigate = useNavigate();
  const { data: coreVersion } = useSWR('mihomoVersion', mihomoVersion);
  const { data: installedVersion } = useSWR('mihomoInstalledVersion', mihomoInstalledVersion);
  const [newVersion, setNewVersion] = useState('');
  const [changelog, setChangelog] = useState('');
  const [openUpdate, setOpenUpdate] = useState(false);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const versionTapCountRef = useRef(0);
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

  const handleVersionClick = (): void => {
    if (showHiddenSettings) return;
    versionTapCountRef.current = Math.min(versionTapCountRef.current + 1, EASTER_EGG_TAP_COUNT);
    if (versionTapCountRef.current >= EASTER_EGG_TAP_COUNT) {
      onUnlockHiddenSettings();
    }
  };

  return (
    <>
      {openUpdate && (
        <UpdaterModal
          onClose={() => setOpenUpdate(false)}
          version={newVersion}
          changelog={changelog}
          updateStatus={updateStatus}
          onCancel={handleCancelUpdate}
        />
      )}
      {confirmOpen && (
        <ConfirmModal
          onChange={setConfirmOpen}
          title={t('settings.actions.confirmReset')}
          description={
            <>
              {t('settings.actions.resetWarning')}
              <span className='text-red-500'>{t('settings.actions.cannotUndo')}</span>
            </>
          }
          confirmText={t('settings.actions.confirmDelete')}
          cancelText={t('common.cancel')}
          onConfirm={resetAppConfig}
        />
      )}
      <SettingCard>
        <SettingItem title={t('settings.actions.checkUpdate')} divider>
          <Button
            size='sm'
            disabled={checkingUpdate}
            onClick={async () => {
              try {
                setCheckingUpdate(true);
                const version = await checkUpdate();
                if (version) {
                  setNewVersion(version.version);
                  setChangelog(version.changelog);
                  setOpenUpdate(true);
                } else {
                  new window.Notification(t('settings.actions.alreadyLatest'), {
                    body: t('settings.actions.noNeedUpdate'),
                  });
                }
              } catch (e) {
                toast.error(`${e}`);
              } finally {
                setCheckingUpdate(false);
              }
            }}
          >
            {t('settings.actions.checkUpdate')}
          </Button>
        </SettingItem>
        <SettingItem
          title={t('settings.actions.resetApp')}
          actions={
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size='icon-sm' variant='ghost'>
                  <MessageCircleQuestionMark className='text-lg' />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('settings.actions.resetAppHelp')}</TooltipContent>
            </Tooltip>
          }
          divider
        >
          <Button size='sm' onClick={() => setConfirmOpen(true)}>
            {t('settings.actions.resetApp')}
          </Button>
        </SettingItem>
        {showHiddenSettings && (
          <SettingItem
            title={t('settings.actions.clearCache')}
            actions={
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button size='icon-sm' variant='ghost'>
                    <MessageCircleQuestionMark className='text-lg' />
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{t('settings.actions.clearCacheHelp')}</TooltipContent>
              </Tooltip>
            }
            divider
          >
            <Button size='sm' onClick={() => localStorage.clear()}>
              {t('settings.actions.clearCache')}
            </Button>
          </SettingItem>
        )}
        {showHiddenSettings && (
          <SettingItem title='Debug Info' divider>
            <Button
              size='sm'
              onClick={async () => {
                try {
                  const info = await debugInfo();
                  console.log('[DEBUG INFO]', info);
                  toast.info('Debug info logged to console (F12)');
                } catch (e) {
                  toast.error(`${e}`);
                }
              }}
            >
              Dump Debug
            </Button>
          </SettingItem>
        )}
        <SettingItem
          title={t('settings.actions.quitKeepCore')}
          actions={
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size='icon-sm' variant='ghost'>
                  <MessageCircleQuestionMark className='text-lg' />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('settings.actions.quitKeepCoreHelp')}</TooltipContent>
            </Tooltip>
          }
          divider
        >
          <Button size='sm' onClick={quitWithoutCore}>
            {t('common.quit')}
          </Button>
        </SettingItem>
        <SettingItem title={t('settings.actions.quitApp')} divider>
          <Button size='sm' onClick={quitApp}>
            {t('settings.actions.quitApp')}
          </Button>
        </SettingItem>
        <SettingItem
          title={t('settings.actions.mihomoVersion')}
          actions={
            <Tooltip>
              <TooltipTrigger asChild>
                <Button size='icon-sm' variant='ghost' onClick={() => navigate('/mihomo')}>
                  <Settings className='text-lg' />
                </Button>
              </TooltipTrigger>
              <TooltipContent>{t('settings.actions.mihomoSettings')}</TooltipContent>
            </Tooltip>
          }
          divider
        >
          <div>{coreVersion?.version || installedVersion || '...'}</div>
        </SettingItem>
        <SettingItem title={t('settings.actions.appVersion')}>
          <button type='button' className='select-none' onClick={handleVersionClick}>
            v{version}
          </button>
        </SettingItem>
      </SettingCard>
    </>
  );
};

export default Actions;
