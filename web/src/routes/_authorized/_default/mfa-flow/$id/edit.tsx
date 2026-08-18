import { createFileRoute, notFound, redirect } from '@tanstack/react-router';
import axios from 'axios';
import Skeleton from 'react-loading-skeleton';
import { MfaFormPage } from '../../../../../pages/MfaPage/MfaFormPage';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import { Snackbar } from '../../../../../shared/defguard-ui/providers/snackbar/snackbar';

export const Route = createFileRoute('/_authorized/_default/mfa-flow/$id/edit')({
  loader: async ({ params }) => {
    const id = Number(params.id);
    if (!Number.isSafeInteger(id) || id <= 0) throw notFound();

    try {
      return (await api.mfaFlow.get(id)).data;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) throw notFound();
      Snackbar.error(m.error_unknown());
      throw redirect({
        to: '/mfa',
        replace: true,
      });
    }
  },
  pendingComponent: () => <Skeleton height={500} />,
  component: RouteComponent,
});

function RouteComponent() {
  const flow = Route.useLoaderData();
  return <MfaFormPage flow={flow} />;
}
