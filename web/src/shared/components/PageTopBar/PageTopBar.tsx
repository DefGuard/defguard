import { Divider } from '../../defguard-ui/components/Divider/Divider';
import './style.scss';
import { IconButton } from '../../defguard-ui/components/IconButton/IconButton';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import { useApp } from '../../hooks/useApp';
import { useAuth } from '../../hooks/useAuth';
import { TopBarLicense } from './components/TopBarLicense/TopBarLicense';
import { TopBarLicenseExpiration } from './components/TopBarLicenseExpiration/TopBarLicenseExpiration';
import { TopBarProfile } from './components/TopBarProfile/TopBarProfile';

type Props = {
  title: string;
  navOpen: boolean;
};

export const PageTopBar = ({ title, navOpen }: Props) => {
  const isAdmin = useAuth((s) => s.isAdmin);
  return (
    <div className="page-top-bar">
      {!navOpen && isAdmin && (
        <>
          <IconButton
            onClick={() => {
              useApp.setState({
                navigationOpen: true,
              });
            }}
            icon="hamburger"
          />
          <SizedBox height={1} width={ThemeSpacing.Xl} />
        </>
      )}
      <p className="page-title">{title}</p>
      <div className="right">
        {isAdmin && (
          <>
            <TopBarLicenseExpiration />
            <TopBarLicense />
          </>
        )}
        <Divider orientation="vertical" />
        <TopBarProfile />
      </div>
    </div>
  );
};
