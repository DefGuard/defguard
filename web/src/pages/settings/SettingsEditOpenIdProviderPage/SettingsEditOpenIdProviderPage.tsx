import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useRouter } from '@tanstack/react-router';
import { useCallback, useMemo } from 'react';
import api from '../../../shared/api/api';
import type { AddOpenIdProvider } from '../../../shared/api/types';
import { EditPage } from '../../../shared/components/EditPage/EditPage';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getExternalProviderQueryOptions } from '../../../shared/query';
import { ExternalProvider } from '../shared/types';
import { EditCustomProviderForm } from './form/EditCustomProviderForm';
import { EditGoogleProviderForm } from './form/EditGoogleProviderForm';
import { EditJumpCloudProviderForm } from './form/EditJumpCloudProviderForm';
import { EditMicrosoftProviderForm } from './form/EditMicrosoftProviderForm';
import { EditOktaProviderForm } from './form/EditOktaProviderForm';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'openid',
    }}
    key={0}
  >
    External Identity providers
  </Link>,
  <Link to="/settings/edit-openid" key={1}>
    Edit
  </Link>,
];

export const SettingsEditOpenIdProviderPage = () => {
  const router = useRouter();
  const { data } = useSuspenseQuery(getExternalProviderQueryOptions);

  const formData = useMemo(() => {
    if (isPresent(data?.provider)) {
      return { ...data.provider, ...data.settings };
    }
  }, [data]);

  const { mutateAsync } = useMutation({
    mutationFn: api.openIdProvider.editOpenIdProvider,
    onSuccess: () => {
      router.history.back();
    },
    meta: {
      invalidate: [['settings'], ['info'], ['openid']],
    },
  });

  const { mutateAsync: deleteProvider, isPending: deletePending } = useMutation({
    mutationFn: api.openIdProvider.deleteOpenIdProvider,
    onSuccess: () => {
      router.history.back();
    },
    meta: {
      invalidate: [['settings'], ['info'], ['openid']],
    },
  });

  const handleSubmit = useCallback(
    async (values: Partial<AddOpenIdProvider>) => {
      if (isPresent(formData)) {
        await mutateAsync({ ...formData, ...values });
      }
    },
    [formData, mutateAsync],
  );

  if (!formData) return null;

  return (
    <EditPage
      id="edit-openid-provider-page"
      pageTitle={'Settings'}
      formTitle={'Edit external OpenID provider'}
      links={breadcrumbs}
    >
      {formData.name === ExternalProvider.Google && (
        <EditGoogleProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === ExternalProvider.Microsoft && (
        <EditMicrosoftProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === ExternalProvider.Okta && (
        <EditOktaProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === ExternalProvider.JumpCloud && (
        <EditJumpCloudProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === ExternalProvider.Custom && (
        <EditCustomProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
    </EditPage>
  );
};
