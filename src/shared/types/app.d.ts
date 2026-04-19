interface AppVersion {
  version: string;
  changelog: string;
}

interface ISysProxyConfig {
  enable: boolean;
  host?: string;
  mode?: SysProxyMode;
  bypass?: string[];
  pacScript?: string;
  settingMode?: 'exec' | 'service';
}

interface IHost {
  domain: string;
  value: string | string[];
}

interface AppConfig {
  core: 'mihomo' | 'mihomo-alpha' | 'system';
  systemCorePath?: string;
  corePermissionMode?: 'elevated' | 'service';
  proxyDisplayOrder: 'default' | 'delay' | 'name';
  proxyDisplayLayout: 'hidden' | 'single' | 'double';
  groupDisplayLayout: 'hidden' | 'single' | 'double';
  envType?: ('bash' | 'cmd' | 'powershell' | 'nushell')[];
  proxyCols: 'auto' | '1' | '2' | '3' | '4';
  connectionDirection: 'asc' | 'desc';
  connectionOrderBy: 'time' | 'upload' | 'download' | 'uploadSpeed' | 'downloadSpeed' | 'process';
  connectionListMode?: 'classic' | 'process';
  connectionViewMode?: 'list' | 'table';
  connectionTableColumns?: string[];
  connectionTableColumnWidths?: Record<string, number>;
  connectionTableSortColumn?: string;
  connectionTableSortDirection?: 'asc' | 'desc';
  connectionInterval?: number;
  disableTray?: boolean;
  proxyInTray: boolean;
  appTheme: AppTheme;
  autoCheckUpdate: boolean;
  silentStart: boolean;
  autoCloseConnection: boolean;
  expandProxyGroups?: boolean;
  sysProxy: ISysProxyConfig;
  maxLogDays: number;
  userAgent?: string;
  delayTestConcurrency?: number;
  delayTestUrl?: string;
  delayTestTimeout?: number;
  controlDns?: boolean;
  controlSniff?: boolean;
  controlTun?: boolean;
  hosts: IHost[];
  showWindowShortcut?: string;
  triggerSysProxyShortcut?: string;
  triggerTunShortcut?: string;
  ruleModeShortcut?: string;
  globalModeShortcut?: string;
  directModeShortcut?: string;
  restartAppShortcut?: string;
  quitWithoutCoreShortcut?: string;
  affectVPNConnections?: boolean;
  networkDetection?: boolean;
  networkDetectionBypass?: string[];
  networkDetectionInterval?: number;
  displayIcon?: boolean;
  displayAppName?: boolean;
  alwaysOnTop?: boolean;
}

interface ProfileConfig {
  current?: string;
  items: ProfileItem[];
}

interface ProfileItem {
  id: string;
  type: 'remote' | 'local';
  name: string;
  url?: string;
  ua?: string;
  file?: string;
  verify?: boolean;
  interval?: number;
  home?: string;
  updated?: number;
  useProxy?: boolean;
  extra?: SubscriptionUserInfo;
  locked?: boolean;
  autoUpdate?: boolean;
  announce?: string;
  logo?: string;
  supportUrl?: string;
}

interface SubscriptionUserInfo {
  upload: number;
  download: number;
  total: number;
  expire: number;
}
