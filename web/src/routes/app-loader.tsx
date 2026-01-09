import { createFileRoute } from '@tanstack/react-router';
import { AppLoaderPage } from '../pages/AppLoaderPage/AppLoaderPage';

export const Route = createFileRoute('/app-loader')({
  component: AppLoaderPage,
});
