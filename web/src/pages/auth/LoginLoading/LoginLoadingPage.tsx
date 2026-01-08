import './style.scss';
import { LoginPage } from '../../../shared/components/LoginPage/LoginPage';
import { LoaderSpinner } from '../../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';

export const LoginLoadingPage = () => {
  return (
    <LoginPage id="login-loading-page">
      <div className="loader-track">
        <LoaderSpinner size={64} variant="primary" />
      </div>
    </LoginPage>
  );
};
