import { LoaderSpinner } from '../../shared/defguard-ui/components/LoaderSpinner/LoaderSpinner';

export const AppLoaderPage = () => {
  return (
    <div id="app-loader-page">
      <LoaderSpinner size={128} variant="primary" />
    </div>
  );
};
