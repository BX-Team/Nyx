import { invoke } from '@tauri-apps/api/core';

interface TitleBarOverlayOptions {
  color?: string;
  symbolColor?: string;
  height?: number;
}

export async function mihomoVersion(): Promise<ControllerVersion> {
  return invoke('mihomo_version');
}

export async function mihomoInstalledVersion(): Promise<string> {
  return invoke('mihomo_installed_version');
}

export async function mihomoConfig(): Promise<ControllerConfigs> {
  return invoke('mihomo_config');
}

export async function mihomoCloseConnection(id: string): Promise<void> {
  return invoke('mihomo_close_connection', { id });
}

export async function mihomoCloseAllConnections(name?: string): Promise<void> {
  return invoke('mihomo_close_all_connections', { name });
}

export async function mihomoRules(): Promise<ControllerRules> {
  return invoke('mihomo_rules');
}

export async function mihomoProxies(): Promise<ControllerProxies> {
  return invoke('mihomo_proxies');
}

export async function mihomoGroups(): Promise<ControllerMixedGroup[]> {
  return invoke('mihomo_groups');
}

export async function mihomoProxyProviders(): Promise<ControllerProxyProviders> {
  return invoke('mihomo_proxy_providers');
}

export async function mihomoUpdateProxyProviders(name: string): Promise<void> {
  return invoke('mihomo_update_proxy_providers', { name });
}

export async function mihomoRuleProviders(): Promise<ControllerRuleProviders> {
  return invoke('mihomo_rule_providers');
}

export async function mihomoUpdateRuleProviders(name: string): Promise<void> {
  return invoke('mihomo_update_rule_providers', { name });
}

export async function mihomoChangeProxy(group: string, proxy: string): Promise<ControllerProxiesDetail> {
  return invoke('mihomo_change_proxy', { group, proxy });
}

export async function mihomoUnfixedProxy(group: string): Promise<ControllerProxiesDetail> {
  return invoke('mihomo_unfixed_proxy', { group });
}

export async function mihomoUpgradeGeo(): Promise<void> {
  return invoke('mihomo_upgrade_geo');
}

export async function mihomoUpgradeUI(): Promise<void> {
  return invoke('mihomo_upgrade_ui');
}

export async function mihomoUpgrade(core?: 'mihomo' | 'mihomo-alpha' | 'system'): Promise<void> {
  return invoke('mihomo_upgrade', { core });
}

export async function mihomoProxyDelay(proxy: string, url?: string): Promise<ControllerProxiesDelay> {
  return invoke('mihomo_proxy_delay', { proxy, url });
}

export async function mihomoGroupDelay(group: string, url?: string): Promise<ControllerGroupDelay> {
  return invoke('mihomo_group_delay', { group, url });
}

export async function patchMihomoConfig(patch: Partial<MihomoConfig>): Promise<void> {
  return invoke('patch_mihomo_config', { patch });
}

export async function checkAutoRun(): Promise<boolean> {
  return invoke('check_auto_run');
}

export async function enableAutoRun(): Promise<void> {
  return invoke('enable_auto_run');
}

export async function disableAutoRun(): Promise<void> {
  return invoke('disable_auto_run');
}

export async function getAppConfig(force = false): Promise<AppConfig> {
  return invoke('get_app_config', { force });
}

export async function patchAppConfig(patch: Partial<AppConfig>): Promise<void> {
  return invoke('patch_app_config', { config: patch });
}

export async function getControledMihomoConfig(force = false): Promise<Partial<MihomoConfig>> {
  return invoke('get_controled_mihomo_config', { force });
}

export async function patchControledMihomoConfig(patch: Partial<MihomoConfig>): Promise<void> {
  return invoke('patch_controled_mihomo_config', { config: patch });
}

export async function getProfileConfig(force = false): Promise<ProfileConfig> {
  return invoke('get_profile_config', { force });
}

export async function setProfileConfig(config: ProfileConfig): Promise<void> {
  return invoke('set_profile_config', { config });
}

export async function getCurrentProfileItem(): Promise<ProfileItem> {
  return invoke('get_current_profile_item');
}

export async function getProfileItem(id: string | undefined): Promise<ProfileItem> {
  return invoke('get_profile_item', { id });
}

export async function changeCurrentProfile(id: string): Promise<void> {
  return invoke('change_current_profile', { id });
}

export async function reloadCurrentProfile(): Promise<void> {
  return invoke('reload_current_profile');
}

export async function addProfileItem(item: Partial<ProfileItem>): Promise<void> {
  return invoke('add_profile_item', { item });
}

export async function removeProfileItem(id: string): Promise<void> {
  return invoke('remove_profile_item', { id });
}

export async function updateProfileItem(item: ProfileItem): Promise<void> {
  return invoke('update_profile_item', { item });
}

export async function getProfileStr(id: string): Promise<string> {
  return invoke('get_profile_str', { id });
}

export async function getFileStr(id: string): Promise<string> {
  return invoke('get_file_str', { path: id });
}

export async function setFileStr(id: string, str: string): Promise<void> {
  return invoke('set_file_str', { path: id, str });
}

export async function getRuleStr(id: string): Promise<string> {
  return invoke('get_rule_str', { id });
}

export async function setRuleStr(id: string, str: string): Promise<void> {
  return invoke('set_rule_str', { id, str });
}

export async function convertMrsRuleset(path: string, behavior: string): Promise<string> {
  return invoke('convert_mrs_ruleset', { path, behavior });
}

export async function setProfileStr(id: string, str: string): Promise<void> {
  return invoke('set_profile_str', { id, str });
}

export async function restartCore(): Promise<void> {
  return invoke('restart_core');
}

export async function restartMihomoConnections(): Promise<void> {
  return invoke('restart_mihomo_connections');
}

export async function triggerSysProxy(enable: boolean, affectVpnConnections: boolean): Promise<void> {
  return invoke('trigger_sys_proxy', { enable, affectVpnConnections });
}

export async function manualGrantCorePermition(cores?: ('mihomo' | 'mihomo-alpha')[]): Promise<void> {
  return invoke('manual_grant_core_permition', { cores });
}

export async function checkCorePermission(): Promise<{ mihomo: boolean; 'mihomo-alpha': boolean }> {
  return invoke('check_core_permission');
}

export async function needsFirstRunAdmin(): Promise<boolean> {
  return invoke('needs_first_run_admin');
}

export async function checkFirstRun(): Promise<boolean> {
  return invoke('check_first_run');
}

export async function isAdmin(): Promise<boolean> {
  return invoke('is_admin');
}

export async function restartAsAdmin(): Promise<void> {
  return invoke('restart_as_admin');
}

export async function revokeCorePermission(cores?: ('mihomo' | 'mihomo-alpha')[]): Promise<void> {
  return invoke('revoke_core_permission', { cores });
}

export async function serviceStatus(): Promise<'running' | 'stopped' | 'not-installed' | 'unknown'> {
  return invoke('service_status');
}

export async function testServiceConnection(): Promise<boolean> {
  return invoke('test_service_connection');
}

export async function initService(): Promise<void> {
  return invoke('init_service');
}

export async function installService(): Promise<void> {
  return invoke('install_service');
}

export async function uninstallService(): Promise<void> {
  return invoke('uninstall_service');
}

export async function startService(): Promise<void> {
  return invoke('start_service');
}

export async function restartService(): Promise<void> {
  return invoke('restart_service');
}

export async function stopService(): Promise<void> {
  return invoke('stop_service');
}

export async function findSystemMihomo(): Promise<string[]> {
  return invoke('find_system_mihomo');
}

export async function checkElevateTask(): Promise<boolean> {
  return invoke('check_elevate_task');
}

export async function createElevateTask(): Promise<void> {
  return invoke('create_elevate_task');
}

export async function deleteElevateTask(): Promise<void> {
  return invoke('delete_elevate_task');
}

export async function getFilePath(ext: string[]): Promise<string | undefined> {
  return invoke('get_file_path', { ext: ext[0] ?? '' });
}

export async function readTextFile(filePath: string): Promise<string> {
  return invoke('read_text_file', { filePath });
}

export async function getRuntimeConfigStr(): Promise<string> {
  return invoke('get_runtime_config_str');
}

export async function getRawProfileStr(): Promise<string> {
  return invoke('get_raw_profile_str');
}

export async function getCurrentProfileStr(): Promise<string> {
  return invoke('get_current_profile_str');
}

export async function getRuntimeConfig(): Promise<MihomoConfig> {
  return invoke('get_runtime_config');
}

export async function checkUpdate(): Promise<AppVersion | undefined> {
  return invoke('check_update');
}

export async function downloadAndInstallUpdate(version: string): Promise<void> {
  return invoke('download_and_install_update', { version });
}

export async function cancelUpdate(): Promise<void> {
  return invoke('cancel_update');
}

export async function getVersion(): Promise<string> {
  return invoke('get_version');
}

export async function openUWPTool(): Promise<void> {
  return invoke('open_uwp_tool');
}

export async function setupFirewall(): Promise<void> {
  return invoke('setup_firewall');
}

export async function getInterfaces(): Promise<Record<string, NetworkInterfaceInfo[]>> {
  return invoke('get_interfaces');
}

export async function setTitleBarOverlay(overlay: TitleBarOverlayOptions): Promise<void> {
  return invoke('set_title_bar_overlay', {
    color: overlay.color,
    symbolColor: overlay.symbolColor,
  });
}

export async function setAlwaysOnTop(alwaysOnTop: boolean): Promise<void> {
  return invoke('set_always_on_top', { value: alwaysOnTop });
}

export async function isAlwaysOnTop(): Promise<boolean> {
  return invoke('is_always_on_top');
}

export async function relaunchApp(): Promise<void> {
  return invoke('relaunch_app');
}

export async function quitWithoutCore(): Promise<void> {
  return invoke('quit_without_core');
}

export async function quitApp(): Promise<void> {
  return invoke('quit_app');
}

export async function notDialogQuit(): Promise<void> {
  return invoke('not_dialog_quit');
}

export async function showTrayIcon(): Promise<void> {
  return invoke('show_tray_icon');
}

export async function closeTrayIcon(): Promise<void> {
  return invoke('close_tray_icon');
}

export async function updateTrayIcon(): Promise<void> {
  return invoke('update_tray_icon');
}

export async function setDockVisible(visible: boolean): Promise<void> {
  return invoke('set_dock_visible', { visible });
}

export async function showMainWindow(): Promise<void> {
  return invoke('show_main_window');
}

export async function closeMainWindow(): Promise<void> {
  return invoke('close_main_window');
}

export async function triggerMainWindow(): Promise<void> {
  return invoke('trigger_main_window');
}

export async function openFile(id: string): Promise<void> {
  return invoke('open_file', { id });
}

export async function openDevTools(): Promise<void> {
  return invoke('open_dev_tools');
}

export async function resetAppConfig(): Promise<void> {
  return invoke('reset_app_config');
}

export async function debugInfo(): Promise<Record<string, unknown>> {
  return invoke('debug_info');
}

export async function getUserAgent(): Promise<string> {
  return invoke('get_user_agent');
}

export async function getAppName(appPath: string): Promise<string> {
  return invoke('get_app_name', { appPath });
}

export async function getImageDataURL(url: string): Promise<string> {
  return invoke('get_image_data_url', { url });
}

export async function getIconDataURL(appPath: string): Promise<string> {
  return invoke('get_icon_data_url', { appPath });
}

export async function startNetworkDetection(): Promise<void> {
  return invoke('start_network_detection');
}

export async function stopNetworkDetection(): Promise<void> {
  return invoke('stop_network_detection');
}

export async function registerShortcut(oldShortcut: string, newShortcut: string, action: string): Promise<boolean> {
  await invoke('register_shortcut', { oldShortcut, newShortcut, action });
  return true;
}

export async function copyEnv(type: 'bash' | 'cmd' | 'powershell' | 'nushell'): Promise<void> {
  return invoke('copy_env', { envType: type });
}

async function alert<T>(msg: T): Promise<void> {
  const { toast } = await import('sonner');
  const msgStr = typeof msg === 'string' ? msg : JSON.stringify(msg);
  toast.error(msgStr);
}

window.alert = alert;
