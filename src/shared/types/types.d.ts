type OutboundMode = 'rule' | 'global' | 'direct';
type LogLevel = 'info' | 'debug' | 'warning' | 'error' | 'silent';
type SysProxyMode = 'auto' | 'manual';
type AppTheme = 'system' | 'light' | 'dark';

type MihomoGroupType = 'Selector' | 'Fallback' | 'URLTest' | 'LoadBalance' | 'Relay';
type MihomoProxyType =
  | 'Direct'
  | 'Reject'
  | 'RejectDrop'
  | 'Compatible'
  | 'Pass'
  | 'Dns'
  | 'Relay'
  | 'Selector'
  | 'Fallback'
  | 'URLTest'
  | 'LoadBalance'
  | 'Shadowsocks'
  | 'ShadowsocksR'
  | 'Snell'
  | 'Socks5'
  | 'Http'
  | 'Vmess'
  | 'Vless'
  | 'Trojan'
  | 'Hysteria'
  | 'Hysteria2'
  | 'WireGuard'
  | 'Tuic'
  | 'Ssh'
  | 'Mieru'
  | 'AnyTLS'
  | 'Sudoku';
type TunStack = 'gvisor' | 'mixed' | 'system';
type FindProcessMode = 'off' | 'strict' | 'always';
type DnsMode = 'normal' | 'fake-ip' | 'redir-host';
type FilterMode = 'blacklist' | 'whitelist';
type NetworkInterfaceInfo = os.NetworkInterfaceInfo;
type Fingerprints =
  | ''
  | 'random'
  | 'randomized'
  | 'chrome'
  | 'chrome_psk'
  | 'chrome_psk_shuffle'
  | 'chrome_padding_psk_shuffle'
  | 'chrome_pq'
  | 'chrome_pq_psk'
  | 'firefox'
  | 'safari'
  | 'ios'
  | 'android'
  | 'edge'
  | '360'
  | 'qq';
