import { t } from 'i18next';
import isCidr from 'is-cidr';
import * as isIp from 'is-ip';

export type ValidationResult = { ok: boolean; error?: string };

export const isIPv4 = (ip: string): ValidationResult => {
  if (!ip) return { ok: false, error: t('common.ipAddressCannotBeEmpty') };
  try {
    return isIp.isIPv4(ip) ? { ok: true } : { ok: false, error: t('common.invalidIPv4') };
  } catch (_e) {
    return { ok: false, error: t('validation.errorParsingIpAddress') };
  }
};

export const isIPv6 = (ip: string): ValidationResult => {
  if (!ip) return { ok: false, error: t('common.ipAddressCannotBeEmpty') };
  try {
    return isIp.isIPv6(ip) ? { ok: true } : { ok: false, error: t('common.invalidIPv6') };
  } catch (_e) {
    return { ok: false, error: t('validation.errorParsingIpAddress') };
  }
};

export const isValidIPv4Cidr = (s: string | undefined): ValidationResult => {
  if (!s || s.trim() === '') return { ok: true };
  const v = s.trim();
  try {
    const r = isCidr(v);
    if (r === 4) return { ok: true };
    if (r === 6) return { ok: false, error: t('validation.expectedIPv4Cidr') };
    return { ok: false, error: t('validation.invalidCidrExample', { example: '198.18.0.1/16' }) };
  } catch (_e) {
    return { ok: false, error: t('validation.errorParsingCidr') };
  }
};

export const isValidIPv6Cidr = (s: string | undefined): ValidationResult => {
  if (!s || s.trim() === '') return { ok: true };
  const v = s.trim();
  try {
    const r = isCidr(v);
    if (r === 6) return { ok: true };
    if (r === 4) return { ok: false, error: t('validation.expectedIPv6Cidr') };
    return { ok: false, error: t('validation.invalidCidrExample', { example: 'fc00::/18' }) };
  } catch (_e) {
    return { ok: false, error: t('validation.errorParsingCidr') };
  }
};

export const isValidPort = (s: string): ValidationResult => {
  if (!/^\d+$/.test(s)) return { ok: false, error: t('common.invalidPort') };
  const p = Number(s);
  return p >= 1 && p <= 65535 ? { ok: true } : { ok: false, error: t('common.portRange') };
};

export const isValidListenAddress = (s: string | undefined): ValidationResult => {
  if (!s || s.trim() === '') return { ok: true };
  const v = s.trim();
  if (v.startsWith(':')) {
    return isValidPort(v.slice(1));
  }
  const idx = v.lastIndexOf(':');
  if (idx === -1) return { ok: false, error: t('validation.missingPort') };
  const host = v.slice(0, idx);
  const port = v.slice(idx + 1);
  if (!isValidPort(port)) return { ok: false, error: t('validation.invalidPortNumber') };
  if (host.startsWith('[') && host.endsWith(']')) {
    const inner = host.slice(1, -1);
    return isIPv6(inner);
  }
  if (/^[0-9a-zA-Z-.]+$/.test(host)) {
    if (/^[0-9.]+$/.test(host)) {
      return isIPv4(host);
    }
    return /^[a-zA-Z0-9-.]+$/.test(host) ? { ok: true } : { ok: false, error: t('validation.invalidHostnameChars') };
  }
  return { ok: false, error: t('validation.invalidHostnameChars') };
};

export const isValidDomainWildcard = (s: string | undefined): ValidationResult => {
  if (!s || s.trim() === '') return { ok: false, error: t('common.cannotBeEmpty') };
  const v = s.trim();
  if (v.startsWith('rule-set:') || v.startsWith('geosite:')) {
    const rest = v.split(':')[1];
    if (!!rest && rest.length > 0) return { ok: true };
    return { ok: false, error: t('validation.ruleSetNameCannotBeEmpty') };
  }
  if (v === '*') return { ok: true };

  if (v.startsWith('+.')) {
    const domain = v.slice(2);
    if (/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(domain)) return { ok: true };
    return { ok: false, error: t('validation.plusDotDomainInvalid') };
  }

  if (v.startsWith('.')) {
    const domain = v.slice(1);
    if (/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(domain)) return { ok: true };
    return { ok: false, error: t('validation.dotDomainInvalid') };
  }

  if (v.includes('*')) {
    const labels = v.split('.');
    if (labels.every(lab => lab === '*' || /^[a-zA-Z0-9-]+$/.test(lab))) return { ok: true };
    return { ok: false, error: t('validation.wildcardPositionInvalid') };
  }

  if (/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(v)) return { ok: true };
  return { ok: false, error: t('common.invalidDomain') };
};

export const isValidPortRange = (s: string | undefined): boolean => {
  if (!s || s.trim() === '') return false;
  const parts = s
    .split(/[,/]/)
    .map(p => p.trim())
    .filter(Boolean);
  if (parts.length === 0) return false;
  for (const p of parts) {
    if (p.includes('-')) {
      const [a, b] = p.split('-');
      if (!/^\d+$/.test(a) || !/^\d+$/.test(b)) return false;
      const na = Number(a);
      const nb = Number(b);
      if (na < 1 || nb > 65535 || na > nb) return false;
    } else {
      if (!/^\d+$/.test(p)) return false;
      const np = Number(p);
      if (np < 1 || np > 65535) return false;
    }
  }
  return true;
};

export const isValidDnsServer = (s: string | undefined, ipOnly = false): ValidationResult => {
  if (!s || s.trim() === '') return { ok: false, error: t('common.cannotBeEmpty') };
  const v = s.trim();
  const hashIndex = v.indexOf('#');
  const serverPart = hashIndex === -1 ? v : v.slice(0, hashIndex);
  const paramsPart = hashIndex === -1 ? '' : v.slice(hashIndex + 1);

  if (!serverPart) return { ok: false, error: t('validation.serverAddressCannotBeEmpty') };
  if (hashIndex !== -1) {
    if (!paramsPart || paramsPart.trim() === '') {
      return { ok: false, error: t('validation.hashParamsCannotBeEmpty') };
    }
    const boolParams = ['ecs-override', 'h3', 'skip-cert-verify', 'disable-ipv4', 'disable-ipv6'];
    const allowedParams = ['ecs', ...boolParams];

    const params = paramsPart
      .split('&')
      .map(p => p.trim())
      .filter(Boolean);
    for (const param of params) {
      if (param.includes('=')) {
        const [key, value] = param.split('=');
        if (!key || !value) {
          return { ok: false, error: t('validation.invalidParamFormat') };
        }
        if (!/^[a-zA-Z0-9-_]+$/.test(key)) {
          return { ok: false, error: t('validation.invalidParamName', { key }) };
        }
        if (!allowedParams.includes(key)) {
          return {
            ok: false,
            error: t('validation.unsupportedParam', { key, allowed: allowedParams.join(', ') }),
          };
        }
        if (boolParams.includes(key) && value !== 'true' && value !== 'false') {
          return { ok: false, error: t('validation.invalidBooleanParam', { key }) };
        }
        if (key === 'ecs' && !/^[a-zA-Z0-9-_./:]+$/.test(value)) {
          return { ok: false, error: t('validation.invalidParamValue', { value }) };
        }
      } else {
        if (!param || param.trim() === '') {
          return { ok: false, error: t('validation.paramCannotBeEmpty') };
        }
        if (!/^[a-zA-Z0-9\u4e00-\u9fa5\s-_]+$/.test(param)) {
          return {
            ok: false,
            error: t('validation.invalidParamNameWithHint', { param }),
          };
        }
      }
    }
  }

  const lower = serverPart.toLowerCase();

  if (lower === 'system' || lower === 'system://') return { ok: true };

  if (lower.startsWith('dhcp://')) {
    const rest = serverPart.slice('dhcp://'.length);
    if (!rest) return { ok: false, error: t('validation.dhcpInterfaceRequired') };
    if (rest.toLowerCase() === 'system') return { ok: true };
    if (/^[a-zA-Z0-9_.-]+$/.test(rest)) return { ok: true };
    return { ok: false, error: t('validation.dhcpInterfaceInvalid') };
  }

  if (lower.startsWith('rcode://')) {
    const code = lower.slice('rcode://'.length);
    const allowed = new Set(['success', 'format_error', 'server_failure', 'name_error', 'not_implemented', 'refused']);
    return allowed.has(code) ? { ok: true } : { ok: false, error: t('validation.invalidRcodeValue') };
  }

  if (/^https?:\/\//i.test(serverPart)) {
    try {
      const u = new URL(serverPart);
      if (!u.hostname) return { ok: false, error: t('validation.invalidUrlHostname') };
      if (ipOnly) {
        const hostname = u.hostname;
        if (/^[0-9.]+$/.test(hostname)) {
          const r = isIPv4(hostname);
          if (!r.ok) return { ok: false, error: t('common.invalidIPv4') };
          return { ok: true };
        }
        if (hostname.includes(':')) {
          const r = isIPv6(hostname);
          if (!r.ok) return { ok: false, error: t('common.invalidIPv6') };
          return { ok: true };
        }
        return { ok: false, error: t('validation.ipOnlyHost') };
      }
      return { ok: true };
    } catch (_e) {
      return { ok: false, error: t('validation.invalidUrlFormat') };
    }
  }

  const schemeMatch = serverPart.match(/^([a-zA-Z][a-zA-Z0-9+.-]*):\/\/(.+)$/);
  if (schemeMatch) {
    const scheme = schemeMatch[1].toLowerCase();
    const rest = schemeMatch[2];
    if (!['udp', 'tcp', 'tls', 'quic'].includes(scheme)) {
      return { ok: false, error: t('validation.unsupportedScheme', { scheme }) };
    }
    const hostPort = rest.split('/')[0];
    const hpIdx = hostPort.lastIndexOf(':');
    let host = hostPort;
    let portStr: string | undefined;
    if (hpIdx !== -1 && !(hostPort.startsWith('[') && hostPort.includes(']') && hpIdx > hostPort.indexOf(']'))) {
      host = hostPort.slice(0, hpIdx);
      portStr = hostPort.slice(hpIdx + 1);
    }
    if (!host) return { ok: false, error: t('validation.schemeMissingHost', { scheme }) };
    if (/^[0-9.]+$/.test(host)) {
      const r = isIPv4(host);
      if (!r.ok) return { ok: false, error: t('common.invalidIPv4') };
    } else if (host.startsWith('[') && host.endsWith(']')) {
      const inner = host.slice(1, -1);
      const r = isIPv6(inner);
      if (!r.ok) return { ok: false, error: t('common.invalidIPv6') };
    } else {
      if (ipOnly) {
        return { ok: false, error: t('validation.ipOnlyHost') };
      }
      if (!/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(host)) {
        return { ok: false, error: t('validation.invalidHostname') };
      }
    }
    if (portStr) {
      if (!/^[0-9]+$/.test(portStr)) return { ok: false, error: t('validation.invalidPortFormat') };
      const p = Number(portStr);
      if (p < 1 || p > 65535) return { ok: false, error: t('common.portRange') };
    }
    return { ok: true };
  }

  const idx = serverPart.lastIndexOf(':');
  if (idx !== -1 && serverPart.includes(']') === false) {
    const host = serverPart.slice(0, idx);
    const port = serverPart.slice(idx + 1);
    if (!/^[0-9]+$/.test(port)) return { ok: false, error: t('validation.invalidPortFormat') };
    if (!host) return { ok: false, error: t('validation.hostCannotBeEmpty') };
    if (/^[0-9.]+$/.test(host)) {
      const r = isIPv4(host);
      return r.ok ? { ok: true } : { ok: false, error: t('common.invalidIPv4') };
    }
    if (host.startsWith('[') && host.endsWith(']')) {
      const inner = host.slice(1, -1);
      const r = isIPv6(inner);
      return r.ok ? { ok: true } : { ok: false, error: t('common.invalidIPv6') };
    }
    if (ipOnly) {
      return { ok: false, error: t('validation.ipOnlyHost') };
    }
    return /^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(host)
      ? { ok: true }
      : { ok: false, error: t('validation.invalidHostname') };
  }

  if (serverPart.startsWith('[') && serverPart.endsWith(']')) {
    const inner = serverPart.slice(1, -1);
    const r = isIPv6(inner);
    return r.ok ? { ok: true } : { ok: false, error: t('common.invalidIPv6') };
  }
  if (/^[0-9.]+$/.test(serverPart)) {
    const r = isIPv4(serverPart);
    return r.ok ? { ok: true } : { ok: false, error: t('common.invalidIPv4') };
  }
  if (ipOnly) {
    return { ok: false, error: t('validation.ipOnlyHost') };
  }
  if (/^[a-zA-Z0-9-]+(\.[a-zA-Z0-9-]+)*$/.test(serverPart)) return { ok: true };
  return { ok: false, error: t('validation.invalidServerAddress') };
};
