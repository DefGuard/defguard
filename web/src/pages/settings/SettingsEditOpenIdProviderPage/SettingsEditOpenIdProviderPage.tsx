import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useRouter } from '@tanstack/react-router';
import { useCallback, useMemo } from 'react';
import api from '../../../shared/api/api';
import { type AddOpenIdProvider, OpenIdProviderKind } from '../../../shared/api/types';
import { EditPage } from '../../../shared/components/EditPage/EditPage';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getExternalProviderQueryOptions } from '../../../shared/query';
import { EditCustomProviderForm } from './form/EditCustomProviderForm';
import { EditGoogleProviderForm } from './form/EditGoogleProviderForm';
import { EditJumpCloudProviderForm } from './form/EditJumpCloudProviderForm';
import { EditMicrosoftProviderForm } from './form/EditMicrosoftProviderForm';
import { EditOktaProviderForm } from './form/EditOktaProviderForm';

const breadcrumbs = [
  <Link to="/settings/openid" key={0}>
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
      links={breadcrumbs}
      headerProps={{
        title: 'Edit external OpenID provider',
      }}
    >
      {formData.name === OpenIdProviderKind.Google && (
        <EditGoogleProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === OpenIdProviderKind.Microsoft && (
        <EditMicrosoftProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === OpenIdProviderKind.Okta && (
        <EditOktaProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === OpenIdProviderKind.JumpCloud && (
        <EditJumpCloudProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => {
            deleteProvider(formData.name);
          }}
          loading={deletePending}
        />
      )}
      {formData.name === OpenIdProviderKind.Custom && (
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
