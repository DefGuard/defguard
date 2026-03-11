import { createFileRoute } from '@tanstack/react-router';
import { OpenIdPage } from '../../../pages/OpenIdPage/OpenIdPage';

export const Route = createFileRoute('/_authorized/_default/openid')({
  component: OpenIdPage,
});
