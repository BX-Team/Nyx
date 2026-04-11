import { getCurrentWindow } from '@tauri-apps/api/window';
import type React from 'react';
import { useEffect, useState } from 'react';
import { useAppConfig } from '@/hooks/use-app-config';

const WindowControls: React.FC = () => {
  const { appConfig } = useAppConfig();
  const { useWindowFrame = false } = appConfig || {};
  const [isMaximized, setIsMaximized] = useState(false);
  const [_isFocused, setIsFocused] = useState(document.hasFocus());

  useEffect(() => {
    if (useWindowFrame) return;

    const appWindow = getCurrentWindow();

    appWindow.isMaximized().then(setIsMaximized);

    const unlistenResize = appWindow.listen('tauri://resize', () => {
      appWindow.isMaximized().then(setIsMaximized);
    });

    const onFocus = (): void => setIsFocused(true);
    const onBlur = (): void => setIsFocused(false);
    window.addEventListener('focus', onFocus);
    window.addEventListener('blur', onBlur);

    return () => {
      unlistenResize.then(f => f());
      window.removeEventListener('focus', onFocus);
      window.removeEventListener('blur', onBlur);
    };
  }, [useWindowFrame]);

  if (useWindowFrame) return null;

  const appWindow = getCurrentWindow();

  const handleMinimize = (): void => {
    appWindow.minimize();
  };
  const handleMaximize = (): void => {
    appWindow.toggleMaximize();
  };
  const handleClose = (): void => {
    appWindow.hide();
  };

  const closeBtn = (
    <button key='close' className='wc-btn wc-close' onClick={handleClose}>
      <svg viewBox='0 0 10 10' fill='none'>
        <path d='M1.5 1.5L8.5 8.5M8.5 1.5L1.5 8.5' stroke='currentColor' strokeWidth='1.3' strokeLinecap='round' />
      </svg>
    </button>
  );

  const minimizeBtn = (
    <button key='minimize' className='wc-btn wc-minimize' onClick={handleMinimize}>
      <svg viewBox='0 0 10 10' fill='none'>
        <path d='M1.5 5H8.5' stroke='currentColor' strokeWidth='1.3' strokeLinecap='round' />
      </svg>
    </button>
  );

  const maximizeBtn = (
    <button key='maximize' className='wc-btn wc-maximize' onClick={handleMaximize}>
      {isMaximized ? (
        <svg viewBox='0 0 10 10' fill='none'>
          <path
            d='M3 1H8.5A.5.5 0 0 1 9 1.5V7'
            stroke='currentColor'
            strokeWidth='1.2'
            strokeLinecap='round'
            strokeLinejoin='round'
          />
          <rect x='1' y='3' width='6' height='6' rx='0.5' stroke='currentColor' strokeWidth='1.2' />
        </svg>
      ) : (
        <svg viewBox='0 0 10 10' fill='none'>
          <rect x='1.5' y='1.5' width='7' height='7' rx='0.5' stroke='currentColor' strokeWidth='1.3' />
        </svg>
      )}
    </button>
  );

  const buttons = [minimizeBtn, maximizeBtn, closeBtn];

  return <div className={`wc-group app-nodrag wc-win`}>{buttons}</div>;
};

export default WindowControls;
