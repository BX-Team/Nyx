import NumberFlow from '@number-flow/react';
import { listen } from '@tauri-apps/api/event';
import { openUrl } from '@tauri-apps/plugin-opener';
import dayjs from 'dayjs';
import {
  ArrowDown,
  ArrowUp,
  ChevronLeft,
  ChevronRight,
  Globe,
  InfinityIcon,
  Pause,
  PlusCircle,
  Power,
  RefreshCcw,
  WifiOff,
} from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { SiTelegram } from 'react-icons/si';
import { useNavigate } from 'react-router-dom';
import { toast } from 'sonner';
import BasePage from '@/components/base/base-page';
import EditInfoModal from '@/components/profiles/edit-info-modal';
import { CharacterMorph } from '@/components/ui/character-morph';
import { Spinner } from '@/components/ui/spinner';
import { useControledMihomoConfig } from '@/hooks/use-controled-mihomo-config';
import { useGroups } from '@/hooks/use-groups';
import { useProfileConfig } from '@/hooks/use-profile-config';
import { cn } from '@/lib/utils';
import { calcTraffic } from '@/utils/calc';
import { isAdmin, restartAsAdmin, serviceStatus, updateTrayIcon } from '@/utils/ipc';

const mainSwitchStorageKey = 'nyx-main-switch-connected';
const mainSwitchEventName = 'nyx-main-switch-status';
const STATS_SIDEBAR_STORAGE_KEY = 'nyx-home-stats-open';

function formatBytes(bytes: number): string {
  if (bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / 1024 ** i).toFixed(i > 1 ? 1 : 0)} ${units[i]}`;
}

const CONNECTION_START_STORAGE_KEY = 'nyx-connection-start-time';

const readStoredStartTime = (): number | null => {
  if (typeof window === 'undefined') return null;
  const raw = window.sessionStorage.getItem(CONNECTION_START_STORAGE_KEY);
  if (!raw) return null;
  const parsed = Number.parseInt(raw, 10);
  return Number.isFinite(parsed) ? parsed : null;
};

const writeStoredStartTime = (value: number | null): void => {
  if (typeof window === 'undefined') return;
  if (value === null) {
    window.sessionStorage.removeItem(CONNECTION_START_STORAGE_KEY);
  } else {
    window.sessionStorage.setItem(CONNECTION_START_STORAGE_KEY, String(value));
  }
};

const readStatsSidebarOpen = (): boolean => {
  if (typeof window === 'undefined') return true;
  const raw = window.localStorage.getItem(STATS_SIDEBAR_STORAGE_KEY);
  if (raw === null) return true;
  return raw === '1';
};

const stripTrailingColon = (value: string): string => value.replace(/[:：]\s*$/, '');

let connectionStartTime: number | null = readStoredStartTime();

const Home: React.FC = () => {
  const { t } = useTranslation();
  const { controledMihomoConfig, patchControledMihomoConfig } = useControledMihomoConfig();
  const { tun } = controledMihomoConfig || {};

  const { profileConfig, addProfileItem } = useProfileConfig();
  const { groups } = useGroups();
  const navigate = useNavigate();
  const hasProfiles = (profileConfig?.items?.length ?? 0) > 0;
  const [showEditModal, setShowEditModal] = useState(false);
  const [editingItem, setEditingItem] = useState<ProfileItem | null>(null);
  const [updating, setUpdating] = useState(false);
  const [statsOpen, setStatsOpen] = useState(readStatsSidebarOpen);

  useEffect(() => {
    if (typeof window === 'undefined') return;
    window.localStorage.setItem(STATS_SIDEBAR_STORAGE_KEY, statsOpen ? '1' : '0');
  }, [statsOpen]);

  const handleAddProfile = (): void => {
    const newProfile: ProfileItem = {
      id: '',
      name: '',
      type: 'remote',
      url: '',
      useProxy: false,
      autoUpdate: true,
    };
    setEditingItem(newProfile);
    setShowEditModal(true);
  };

  const [connectionsInfo, setConnectionsInfo] = useState<ControllerConnections>();

  useEffect(() => {
    const handleConnections = (_e: unknown, info: ControllerConnections): void => {
      setConnectionsInfo(info);
    };
    const u = listen<ControllerConnections>('mihomo-connections', e => handleConnections(null, e.payload));
    return (): void => {
      u.then(f => f());
    };
  }, []);

  const [loading, setLoading] = useState(false);
  const [loadingDirection, setLoadingDirection] = useState<'connecting' | 'disconnecting'>('connecting');

  const [elapsed, setElapsed] = useState(() => {
    if (connectionStartTime !== null) {
      return Math.floor((Date.now() - connectionStartTime) / 1000);
    }
    return 0;
  });

  const isSelected = tun?.enable ?? false;
  const configLoaded = controledMihomoConfig !== undefined;

  useEffect(() => {
    if (!configLoaded) return undefined;
    if (isSelected) {
      const startTime = connectionStartTime ?? Date.now();
      connectionStartTime = startTime;
      writeStoredStartTime(startTime);
      setElapsed(Math.floor((Date.now() - startTime) / 1000));
      const interval = setInterval(() => {
        setElapsed(Math.floor((Date.now() - startTime) / 1000));
      }, 1000);
      return () => clearInterval(interval);
    } else {
      connectionStartTime = null;
      writeStoredStartTime(null);
      setElapsed(0);
      return undefined;
    }
  }, [isSelected, configLoaded]);

  const isDisabled = loading;

  const status = loading
    ? loadingDirection === 'connecting'
      ? t('pages.home.connecting')
      : t('pages.home.disconnecting')
    : isSelected
      ? t('pages.home.connected')
      : t('pages.home.disconnected');
  const statusWidthTexts = [
    t('pages.home.connecting'),
    t('pages.home.disconnecting'),
    t('pages.home.connected'),
    t('pages.home.disconnected'),
  ];
  const showConnectedTimer = !loading && isSelected;

  useEffect(() => {
    window.sessionStorage.setItem(mainSwitchStorageKey, showConnectedTimer ? '1' : '0');
    window.dispatchEvent(
      new CustomEvent(mainSwitchEventName, {
        detail: { connected: showConnectedTimer },
      }),
    );
  }, [showConnectedTimer]);

  const elapsedHours = Math.floor(elapsed / 3600);
  const elapsedMinutes = Math.floor((elapsed % 3600) / 60);
  const elapsedSeconds = elapsed % 60;

  const currentProfile = useMemo(() => {
    if (!profileConfig?.current || !profileConfig?.items) return null;
    return profileConfig.items.find(item => item.id === profileConfig.current) ?? null;
  }, [profileConfig]);

  const handleUpdateProfile = async (): Promise<void> => {
    if (!currentProfile || updating) return;
    setUpdating(true);
    try {
      await addProfileItem(currentProfile);
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setUpdating(false);
    }
  };

  const subscription = currentProfile?.extra;
  const trafficUsed = (subscription?.upload ?? 0) + (subscription?.download ?? 0);
  const trafficTotal = subscription?.total ?? 0;
  const trafficRemaining = trafficTotal > 0 ? trafficTotal - trafficUsed : 0;
  const expireTimestamp = subscription?.expire ?? 0;
  const expireDate = expireTimestamp > 0 ? dayjs.unix(expireTimestamp).format('L') : t('pages.home.never');
  const daysRemaining = expireTimestamp > 0 ? Math.max(0, dayjs.unix(expireTimestamp).diff(dayjs(), 'day')) : 0;

  const firstGroup = groups?.[0];
  const currentProxyType = useMemo(() => {
    if (!firstGroup?.all || !firstGroup.now) return null;
    const proxy = firstGroup.all.find(p => p?.name === firstGroup.now);
    return proxy?.type ?? null;
  }, [firstGroup]);

  const supportUrl = currentProfile?.supportUrl;
  const supportLinkInfo = useMemo(() => {
    if (!supportUrl) return null;
    try {
      const parsed = new URL(supportUrl);
      const normalized = `${parsed.hostname}${parsed.pathname}`.toLowerCase();
      return {
        href: parsed.toString(),
        isTelegram: parsed.protocol === 'tg:' || normalized.includes('t.me') || normalized.includes('telegram'),
      };
    } catch {
      return null;
    }
  }, [supportUrl]);

  const announceLines = useMemo(() => {
    const raw = currentProfile?.announce?.trim();
    if (!raw) return [];
    return raw
      .split(/\r?\n/)
      .map(line => line.replace(/^[\s•·\-*]+/, '').trim())
      .filter(Boolean);
  }, [currentProfile?.announce]);

  const hasSubscriptionStats = subscription !== undefined;
  const hasSidebarContent = hasSubscriptionStats || announceLines.length > 0;
  const sidebarExpanded = statsOpen && hasSidebarContent;

  const indicatorColor = loading ? 'bg-amber-400' : isSelected ? 'bg-stroke-power-on' : 'bg-muted-foreground/60';

  const onValueChange = async (enable: boolean): Promise<void> => {
    setLoading(true);
    setLoadingDirection(enable ? 'connecting' : 'disconnecting');
    try {
      if (enable) {
        const admin = await isAdmin();
        if (!admin) {
          const svcStatus = await serviceStatus();
          if (svcStatus !== 'running') {
            if (import.meta.env.DEV) {
              toast.error(t('pages.home.tunRequiresAdminDev'), { duration: 10000 });
            } else {
              toast.error(t('pages.home.tunRequiresAdmin'), {
                duration: 10000,
                action: {
                  label: t('pages.home.restartAsAdmin'),
                  onClick: () => restartAsAdmin(),
                },
              });
            }
            return;
          }
        }
        try {
          const svcStatus = await serviceStatus();
          if (svcStatus === 'not-installed') {
            toast.warning(t('pages.home.tunServiceNotInstalled'), { duration: 6000 });
          }
        } catch {}
        await patchControledMihomoConfig({
          tun: {
            enable: true,
            stack: 'mixed',
            'auto-route': true,
            'auto-detect-interface': true,
            'dns-hijack': ['any:53'],
          },
          dns: {
            enable: true,
            'enhanced-mode': 'fake-ip',
            'fake-ip-range': '198.18.0.1/16',
            listen: '0.0.0.0:1053',
            nameserver: ['https://dns.alidns.com/dns-query', 'https://doh.pub/dns-query'],
          },
        });
      } else {
        await patchControledMihomoConfig({ tun: { enable: false } });
      }
      await updateTrayIcon();
    } catch (e) {
      toast.error(`${e}`);
    } finally {
      setLoading(false);
    }
  };

  return (
    <BasePage>
      {!hasProfiles ? (
        <div className='h-full w-full flex items-center justify-center'>
          <div className='flex flex-col items-center gap-4 max-w-75 rounded-2xl border border-stroke bg-card/50 backdrop-blur-xl p-8'>
            <WifiOff className='size-16 text-muted-foreground' />
            <h2 className='text-xl font-bold text-foreground'>{t('pages.profiles.emptyTitle')}</h2>
            <p className='text-sm font-medium text-muted-foreground text-center'>
              {t('pages.profiles.emptyDescription')}
            </p>
            <button
              onClick={handleAddProfile}
              className='flex items-center gap-2 rounded-xl border border-stroke bg-gradient-start-power-on/50 backdrop-blur-xl px-6 py-3 text-foreground hover:bg-gradient-start-power-on/40 transition-colors'
            >
              <PlusCircle className='size-5' />
              <span className='text-sm font-medium'>{t('pages.profiles.addProfile')}</span>
            </button>
          </div>
          {showEditModal && editingItem && (
            <EditInfoModal
              item={editingItem}
              isCurrent={false}
              updateProfileItem={async (item: ProfileItem) => {
                await addProfileItem(item);
                setShowEditModal(false);
                setEditingItem(null);
              }}
              onClose={() => {
                setShowEditModal(false);
                setEditingItem(null);
              }}
            />
          )}
        </div>
      ) : (
        <div className='flex h-full gap-3 px-2 pb-2'>
          {/* Main column */}
          <div className='flex flex-col flex-1 min-w-0 gap-3'>
            {/* Profile header bar */}
            {currentProfile && (
              <div className='flex items-center gap-3 rounded-2xl border border-stroke bg-card/50 backdrop-blur-xl pl-3 pr-1 py-1.5 min-h-10'>
                {currentProfile.logo && (
                  <img
                    src={currentProfile.logo}
                    alt=''
                    className='size-6 rounded-full shrink-0'
                    onError={e => {
                      (e.target as HTMLImageElement).style.display = 'none';
                    }}
                  />
                )}
                <span className='font-semibold text-sm truncate'>{currentProfile.name}</span>
                <div className='h-4 w-px bg-stroke shrink-0' />
                <div className='flex items-center gap-1.5 text-sm text-muted-foreground min-w-0'>
                  <span className={cn('size-2 rounded-full shrink-0 transition-colors', indicatorColor)} />
                  <span className='truncate lowercase'>{status}</span>
                </div>
                <div className='ml-auto flex items-center gap-0.5 shrink-0'>
                  {supportLinkInfo && (
                    <button
                      type='button'
                      onClick={() => openUrl(supportLinkInfo.href)}
                      title={t('pages.profiles.support')}
                      className='size-7 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-accent transition-colors'
                    >
                      {supportLinkInfo.isTelegram ? <SiTelegram className='size-4' /> : <Globe className='size-4' />}
                    </button>
                  )}
                  {currentProfile.type === 'remote' && (
                    <button
                      type='button'
                      onClick={handleUpdateProfile}
                      disabled={updating}
                      title={t('common.update')}
                      className='size-7 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-accent transition-colors disabled:opacity-50'
                    >
                      <RefreshCcw className={`size-4 ${updating ? 'animate-spin' : ''}`} />
                    </button>
                  )}
                  {hasSidebarContent && (
                    <button
                      type='button'
                      onClick={() => setStatsOpen(v => !v)}
                      title={t('pages.home.toggleStats')}
                      className='size-7 rounded-lg flex items-center justify-center text-muted-foreground hover:text-foreground hover:bg-accent transition-colors'
                    >
                      {sidebarExpanded ? <ChevronRight className='size-4' /> : <ChevronLeft className='size-4' />}
                    </button>
                  )}
                </div>
              </div>
            )}

            {/* Connection button */}
            <div className='flex flex-col flex-1 items-center justify-center min-h-0'>
              <div className='mb-3 flex h-6 items-center justify-center'>
                <CharacterMorph
                  texts={[status]}
                  reserveTexts={statusWidthTexts}
                  interval={3000}
                  className='h-6 leading-none text-foreground font-semibold uppercase'
                />
              </div>
              <button
                type='button'
                disabled={isDisabled}
                onClick={() => onValueChange(!isSelected)}
                className='relative group transition-transform active:scale-95'
              >
                <div
                  className={`w-32 h-32 rounded-full flex items-center justify-center transition-all duration-300 bg-radial-[at_30%_45%] backdrop-blur-xl border-2 ${
                    isSelected
                      ? 'from-gradient-start-power-on/60 to-gradient-end-power-on/60 border-stroke-power-on'
                      : 'from-gradient-start-power-off/50 to-gradient-end-power-off/50 border-stroke-power-off'
                  } ${loading ? 'animate-none' : ''}`}
                >
                  <div className='relative size-16'>
                    <Spinner
                      className={`absolute inset-0 m-auto size-16 text-[#FAFAFA] transition-all duration-300 ease-out ${
                        loading ? 'opacity-100 scale-100' : 'opacity-0 scale-90'
                      }`}
                    />
                    <Pause
                      className={`absolute inset-0 size-16 transition-all duration-300 ease-out ${
                        !loading && isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-90'
                      }`}
                    />
                    <Power
                      className={`absolute inset-0 size-16 transition-all duration-300 ease-out ${
                        !loading && !isSelected ? 'opacity-100 scale-100' : 'opacity-0 scale-90'
                      }`}
                    />
                  </div>
                </div>
              </button>
              <div className='mt-3 h-8 flex items-center justify-center'>
                <div
                  aria-hidden={!showConnectedTimer}
                  className={`inline-flex items-center gap-0.5 text-base font-bold text-foreground tabular-nums transition-all duration-300 ease-out ${
                    showConnectedTimer ? 'opacity-100 translate-y-0' : 'opacity-0 translate-y-1'
                  }`}
                >
                  <NumberFlow value={elapsedHours} format={{ minimumIntegerDigits: 2, useGrouping: false }} />
                  <span>:</span>
                  <NumberFlow value={elapsedMinutes} format={{ minimumIntegerDigits: 2, useGrouping: false }} />
                  <span>:</span>
                  <NumberFlow value={elapsedSeconds} format={{ minimumIntegerDigits: 2, useGrouping: false }} />
                </div>
              </div>
              <div
                aria-hidden={!showConnectedTimer}
                className={`mt-2 flex items-center gap-4 tabular-nums transition-all duration-300 ease-out ${
                  showConnectedTimer ? 'opacity-100 translate-y-0' : 'opacity-0 -translate-y-1'
                }`}
              >
                <div className='flex items-center gap-1.5 text-sm text-muted-foreground'>
                  <ArrowUp className='size-3.5 text-stroke-power-on' />
                  <span>{calcTraffic(connectionsInfo?.uploadTotal ?? 0)}</span>
                </div>
                <div className='h-3 w-px bg-stroke' />
                <div className='flex items-center gap-1.5 text-sm text-muted-foreground'>
                  <ArrowDown className='size-3.5 text-stroke-power-on' />
                  <span>{calcTraffic(connectionsInfo?.downloadTotal ?? 0)}</span>
                </div>
              </div>
            </div>

            {/* Proxy selector */}
            {firstGroup && (
              <div className='flex flex-col items-center mx-auto w-full max-w-3xs'>
                <button
                  type='button'
                  onClick={() => navigate('/proxies', { state: { fromHome: true } })}
                  className='w-full text-left cursor-pointer rounded-2xl border border-stroke bg-card/50 backdrop-blur-xl hover:bg-accent/40 transition-colors pl-3 pr-1 py-1.5'
                >
                  <div className='flex items-center justify-between gap-2'>
                    <div className='flex flex-col min-w-0 gap-0.5'>
                      <div className='flag-emoji text-sm font-medium truncate'>{firstGroup.now || firstGroup.name}</div>
                      {currentProxyType && (
                        <div className='text-[11px] text-muted-foreground uppercase tracking-wide truncate'>
                          {currentProxyType}
                        </div>
                      )}
                    </div>
                    <ChevronRight className='size-5 text-muted-foreground shrink-0' />
                  </div>
                </button>
              </div>
            )}
          </div>

          {/* Right: stats sidebar */}
          {hasSidebarContent && (
            <aside
              aria-hidden={!sidebarExpanded}
              className={cn(
                'shrink-0 overflow-hidden transition-[width] duration-200 ease-out',
                sidebarExpanded ? 'w-72' : 'w-0',
              )}
            >
              <div className='w-72 h-full flex flex-col gap-3 rounded-2xl border border-stroke bg-card/50 backdrop-blur-xl p-3'>
                {hasSubscriptionStats && (
                  <div>
                    <h3 className='text-[11px] font-semibold text-muted-foreground uppercase tracking-widest mb-2 px-1'>
                      {t('pages.home.statistics')}
                    </h3>
                    <div className='flex flex-col gap-2'>
                      <div className='rounded-xl border border-stroke bg-background/40 p-3'>
                        <div className='text-xs text-muted-foreground'>
                          {stripTrailingColon(t('pages.home.trafficRemaining'))}
                        </div>
                        <div className='text-xl font-bold mt-1 flex items-center gap-1'>
                          {trafficTotal > 0 ? formatBytes(trafficRemaining) : <InfinityIcon className='size-5' />}
                        </div>
                        {trafficTotal > 0 && (
                          <div className='text-[11px] text-muted-foreground mt-1 tabular-nums'>
                            {formatBytes(trafficUsed)} / {formatBytes(trafficTotal)}
                          </div>
                        )}
                      </div>
                      <div className='grid grid-cols-2 gap-2'>
                        <div className='rounded-xl border border-stroke bg-background/40 p-3 min-w-0'>
                          <div className='text-xs text-muted-foreground truncate'>
                            {stripTrailingColon(t('pages.home.daysRemaining'))}
                          </div>
                          <div className='text-base font-bold mt-1 flex items-center'>
                            {expireTimestamp > 0 ? daysRemaining : <InfinityIcon className='size-4' />}
                          </div>
                        </div>
                        <div className='rounded-xl border border-stroke bg-background/40 p-3 min-w-0'>
                          <div className='text-xs text-muted-foreground truncate'>
                            {stripTrailingColon(t('pages.home.expires'))}
                          </div>
                          <div className='text-base font-bold mt-1 truncate'>{expireDate}</div>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {hasSubscriptionStats && announceLines.length > 0 && <div className='h-px bg-stroke shrink-0' />}

                {announceLines.length > 0 && (
                  <div className='flex flex-col min-h-0'>
                    <h3 className='text-[11px] font-semibold text-muted-foreground uppercase tracking-widest mb-2 px-1'>
                      {t('pages.home.subscriptionNews')}
                    </h3>
                    <ul className='space-y-1.5 text-sm overflow-y-auto custom-scrollbar pr-1'>
                      {announceLines.map((line, i) => (
                        <li key={`${i}-${line}`} className='flex gap-2 break-words'>
                          <span className='text-muted-foreground shrink-0 select-none leading-5'>·</span>
                          <span className='whitespace-pre-line leading-5'>{line}</span>
                        </li>
                      ))}
                    </ul>
                  </div>
                )}
              </div>
            </aside>
          )}
        </div>
      )}
    </BasePage>
  );
};

export default Home;
