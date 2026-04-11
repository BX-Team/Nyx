import { ThemeProvider as NextThemesProvider } from 'next-themes';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { HashRouter } from 'react-router-dom';
import { init } from '@/utils/init';
import '@/assets/main.css';
import '@/components/monaco/monaco-workers';
import App from '@/App';
import BaseErrorBoundary from './components/base/base-error-boundary';
import { Toaster } from './components/ui/sonner';
import { TooltipProvider } from './components/ui/tooltip';
import { AppConfigProvider } from './hooks/use-app-config';
import { ControledMihomoConfigProvider } from './hooks/use-controled-mihomo-config';
import { GroupsProvider } from './hooks/use-groups';
import { ProfileConfigProvider } from './hooks/use-profile-config';
import { RulesProvider } from './hooks/use-rules';
import { openDevTools, quitApp } from './utils/ipc';

let F12Count = 0;

init().then(() => {
  document.addEventListener('keydown', e => {
    if (e.ctrlKey && e.key === 'q') {
      e.preventDefault();
      quitApp();
    }
    if (e.key === 'F12') {
      e.preventDefault();
      F12Count++;
      if (F12Count >= 5) {
        openDevTools();
        F12Count = 0;
      }
    }
  });
});

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <NextThemesProvider attribute='class' enableSystem defaultTheme='dark'>
      <TooltipProvider>
        <BaseErrorBoundary>
          <HashRouter>
            <AppConfigProvider>
              <ControledMihomoConfigProvider>
                <ProfileConfigProvider>
                  <GroupsProvider>
                    <RulesProvider>
                      <App />
                      <Toaster richColors position='bottom-right' />
                    </RulesProvider>
                  </GroupsProvider>
                </ProfileConfigProvider>
              </ControledMihomoConfigProvider>
            </AppConfigProvider>
          </HashRouter>
        </BaseErrorBoundary>
      </TooltipProvider>
    </NextThemesProvider>
  </React.StrictMode>,
);
