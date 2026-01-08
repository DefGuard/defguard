import './style.scss';
import { LoginPageLogo } from '../../shared/components/LoginPage/LoginPageLogo';
import { LoaderSpinner } from '../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';

export const AppLoaderPage = () => {
  return (
    <div id="app-loader-page">
      <LoginPageLogo />
      <LoaderSpinner size={64} variant="primary" />
    </div>
  );
};
