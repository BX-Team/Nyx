import { Navigate } from 'react-router-dom';
import Connections from '@/pages/connections';
import DNS from '@/pages/dns';
import Home from '@/pages/home';
import Logs from '@/pages/logs';
import Mihomo from '@/pages/mihomo';
import Profiles from '@/pages/profiles';
import Proxies from '@/pages/proxies';
import Resources from '@/pages/resources';
import Rules from '@/pages/rules';
import Settings from '@/pages/settings';
import Sniffer from '@/pages/sniffer';
import Sysproxy from '@/pages/sysproxy';
import Tun from '@/pages/tun';

const routes = [
  {
    path: '/mihomo',
    element: <Mihomo />,
  },
  {
    path: '/sysproxy',
    element: <Sysproxy />,
  },
  {
    path: '/tun',
    element: <Tun />,
  },
  {
    path: '/proxies',
    element: <Proxies />,
  },
  {
    path: '/rules',
    element: <Rules />,
  },
  {
    path: '/resources',
    element: <Resources />,
  },
  {
    path: '/dns',
    element: <DNS />,
  },
  {
    path: '/sniffer',
    element: <Sniffer />,
  },
  {
    path: '/logs',
    element: <Logs />,
  },
  {
    path: '/connections',
    element: <Connections />,
  },
  {
    path: '/profiles',
    element: <Profiles />,
  },
  {
    path: '/settings',
    element: <Settings />,
  },
  {
    path: '/',
    element: <Navigate to='/home' />,
  },
  {
    path: '/home',
    element: <Home />,
  },
];

export default routes;
