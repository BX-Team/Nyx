import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type React from 'react';
import { useEffect, useRef, useState } from 'react';
import { useLocation, useRoutes } from 'react-router-dom';
import { toast } from 'sonner';
import './i18n';
import { useTranslation } from 'react-i18next';
import useSWR from 'swr';
import AppSidebar from '@/components/app-sidebar';
import ConfirmModal from '@/components/base/base-confirm';
import HwidLimitAlert from '@/components/profiles/hwid-limit-alert';
import { SidebarProvider } from '@/components/ui/sidebar';
import { useAppConfig } from '@/hooks/use-app-config';
import routes from '@/routes';
import { platform } from '@/utils/init';
import {
  checkUpdate,
  getControledMihomoConfig,
  needsFirstRunAdmin,
  patchControledMihomoConfig,
  restartAsAdmin,
  restartCore,
  updateTrayIcon,
} from '@/utils/ipc';

const mainSwitchStorageKey = 'nyx-main-switch-connected';
const mainSwitchEventName = 'nyx-main-switch-status';

const getInitialMainSwitchConnected = (): boolean => {
  if (typeof window === 'undefined') return false;
  return window.sessionStorage.getItem(mainSwitchStorageKey) === '1';
};

const App: React.FC = () => {
  const { t } = useTranslation();
  const { appConfig } = useAppConfig();
  const { autoCheckUpdate } = appConfig || {};
  const location = useLocation();
  const isHome = location.pathname === '/' || location.pathname.includes('/home');
  const page = useRoutes(routes);
  const { data: latest } = useSWR(
    autoCheckUpdate ? ['checkUpdate'] : undefined,
    autoCheckUpdate ? checkUpdate : (): undefined => {},
    {
      refreshInterval: 1000 * 60 * 10,
    },
  );

  const [showQuitConfirm, setShowQuitConfirm] = useState(false);
  const [showProfileInstallConfirm, setShowProfileInstallConfirm] = useState(false);
  const [showAdminRequired, setShowAdminRequired] = useState(false);
  const [showFirstRunPrompt, setShowFirstRunPrompt] = useState(false);
  const profileInstallConfirmedRef = useRef(false);
  const [profileInstallData, setProfileInstallData] = useState<{
    url: string;
    name?: string | null;
  }>();
  const [isMainSwitchConnected, setIsMainSwitchConnected] = useState<boolean>(getInitialMainSwitchConnected);

  useEffect(() => {
    const onMainSwitchStatus = (event: Event): void => {
      const detail = (event as CustomEvent<{ connected?: boolean }>).detail;
      const connected = detail?.connected === true;
      setIsMainSwitchConnected(connected);
      window.sessionStorage.setItem(mainSwitchStorageKey, connected ? '1' : '0');
    };

    window.addEventListener(mainSwitchEventName, onMainSwitchStatus as EventListener);

    return (): void => {
      window.removeEventListener(mainSwitchEventName, onMainSwitchStatus as EventListener);
    };
  }, []);

  useEffect(() => {
    const unlisteners: UnlistenFn[] = [];

    const setup = async (): Promise<void> => {
      unlisteners.push(await listen<void>('show-quit-confirm', () => setShowQuitConfirm(true)));
      unlisteners.push(
        await listen<{ url: string; name?: string | null }>('show-profile-install-confirm', e => {
          profileInstallConfirmedRef.current = false;
          setProfileInstallData(e.payload);
          setShowProfileInstallConfirm(true);
        }),
      );
      unlisteners.push(
        await listen<{ title: string; message: string }>('show-error', e => {
          toast.error(e.payload.title, { description: e.payload.message });
        }),
      );
      unlisteners.push(await listen<void>('needs-admin-setup', () => setShowAdminRequired(true)));
      unlisteners.push(await listen<void>('first-run', () => setShowFirstRunPrompt(true)));
      unlisteners.push(
        await listen<{ name?: string; id?: string }>('profile-installed', e => {
          const name = e.payload?.name?.trim() || t('common.unnamed');
          toast.success(t('modal.profileInstalled', { name }));
        }),
      );
      unlisteners.push(
        await listen<void>('shortcut-trigger-tun', async () => {
          try {
            const config = await getControledMihomoConfig();
            const enable = !(config?.tun?.enable ?? false);
            if (enable) {
              await patchControledMihomoConfig({ tun: { enable }, dns: { enable: true } });
            } else {
              await patchControledMihomoConfig({ tun: { enable } });
            }
            await restartCore();
            await updateTrayIcon();
          } catch (e) {
            console.error('TUN shortcut toggle failed:', e);
          }
        }),
      );
    };

    setup();

    if (platform === 'win32') {
      needsFirstRunAdmin().then(needs => {
        if (needs) setShowAdminRequired(true);
      });
    }

    return (): void => {
      unlisteners.forEach(u => {
        u();
      });
    };
  }, [t]);

  const handleQuitConfirm = (confirmed: boolean): void => {
    setShowQuitConfirm(false);
    if (confirmed) {
      import('@/utils/ipc').then(({ quitApp }) => quitApp());
    }
  };

  const handleProfileInstallConfirm = (_confirmed: boolean): void => {
    setShowProfileInstallConfirm(false);
  };

  return (
    <SidebarProvider
      defaultOpen={false}
      className='relative w-full h-screen overflow-hidden'
      style={{ backgroundColor: '#080F16' }}
    >
      <div
        className={`pointer-events-none absolute inset-0 z-0 transition-[filter,opacity] duration-500 nyx-bg nyx-bg-3 ${
          isHome ? 'opacity-100' : 'opacity-65 blur-3xl'
        }`}
      />
      <div
        className={`pointer-events-none absolute inset-0 z-0 nyx-bg-state-overlay ${isHome ? 'opacity-100' : 'opacity-65 blur-2xl'}`}
      >
        <div
          className={`nyx-bg-state-layer nyx-bg-state-on ${isMainSwitchConnected ? 'nyx-bg-state-visible' : 'nyx-bg-state-hidden'}`}
        />
        <div
          className={`nyx-bg-state-layer nyx-bg-state-off ${isMainSwitchConnected ? 'nyx-bg-state-hidden' : 'nyx-bg-state-visible'}`}
        />
      </div>
      <div className='pointer-events-none absolute inset-0 z-0 bg-[radial-gradient(circle_at_50%_120%,rgba(4,8,14,0.75),rgba(4,8,14,0.05)_52%,transparent_80%)]'></div>
      {showQuitConfirm && (
        <ConfirmModal
          title={t('modal.confirmQuit')}
          description={
            <div>
              <p></p>
              <p className='text-sm text-gray-500 mt-2'>{t('modal.quitWarning')}</p>
              <p className='text-sm text-gray-400 mt-1'>
                {t('modal.quickQuitHint')} {'Ctrl+Q'} {t('modal.canQuitDirectly')}
              </p>
            </div>
          }
          confirmText={t('common.quit')}
          cancelText={t('common.cancel')}
          onChange={open => {
            if (!open) {
              handleQuitConfirm(false);
            }
          }}
          onConfirm={() => handleQuitConfirm(true)}
        />
      )}
      {showProfileInstallConfirm && profileInstallData && (
        <ConfirmModal
          title={t('modal.confirmImportProfile')}
          description={
            <div className='max-w-md'>
              <p className='text-sm text-gray-600 mb-2'>
                {t('modal.nameLabel')}
                {profileInstallData.name || t('common.unnamed')}
              </p>
              <p className='text-sm text-gray-600 mb-2 truncate'>
                {t('modal.linkLabel')}
                {profileInstallData.url}
              </p>
              <p className='text-sm text-orange-500 mt-2 text-balance'>{t('modal.ensureTrustedSource')}</p>
            </div>
          }
          confirmText={t('common.import')}
          cancelText={t('common.cancel')}
          onChange={open => {
            if (!open) {
              handleProfileInstallConfirm(profileInstallConfirmedRef.current);
              profileInstallConfirmedRef.current = false;
            }
          }}
          onConfirm={() => {
            profileInstallConfirmedRef.current = true;
          }}
          className='min-w-lg'
        />
      )}
      {showAdminRequired && (
        <ConfirmModal
          title={t('modal.adminRequired')}
          description={
            <div>
              <p className='text-sm'>{t('modal.adminRequiredDesc')}</p>
            </div>
          }
          confirmText={t('modal.restartAsAdmin')}
          onChange={open => {
            if (!open) {
              setShowAdminRequired(false);
            }
          }}
          onConfirm={async () => {
            await restartAsAdmin();
          }}
          className=''
        />
      )}
      {showFirstRunPrompt && (
        <ConfirmModal
          title={t('modal.firstRunTitle')}
          description={
            <div>
              <p className='text-sm'>{t('modal.firstRunDesc')}</p>
            </div>
          }
          confirmText={t('modal.firstRunInstall')}
          cancelText={t('modal.firstRunLater')}
          onChange={open => {
            if (!open) setShowFirstRunPrompt(false);
          }}
          onConfirm={() => {
            setShowFirstRunPrompt(false);
            import('pubsub-js').then(({ default: PubSub }) => {
              PubSub.publish('open-service-modal');
            });
          }}
        />
      )}
      <HwidLimitAlert />
      <AppSidebar latest={latest} />
      <div className='relative z-10 main grow h-full overflow-y-auto'>{page}</div>
    </SidebarProvider>
  );
};

export default App;
