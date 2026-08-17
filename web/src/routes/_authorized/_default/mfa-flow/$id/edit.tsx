import { createFileRoute, notFound } from '@tanstack/react-router';
import axios from 'axios';
import { MfaFormPage } from '../../../../../pages/MfaPage/MfaFormPage';
import api from '../../../../../shared/api/api';

export const Route = createFileRoute('/_authorized/_default/mfa-flow/$id/edit')({
  loader: async ({ params }) => {
    const id = Number(params.id);
    if (!Number.isSafeInteger(id) || id <= 0) throw notFound();

    try {
      return (await api.mfaFlow.get(id)).data;
    } catch (error) {
      if (axios.isAxiosError(error) && error.response?.status === 404) throw notFound();
      throw error;
    }
  },
  component: RouteComponent,
});

function RouteComponent() {
  const flow = Route.useLoaderData();
  return <MfaFormPage flow={flow} />;
}
