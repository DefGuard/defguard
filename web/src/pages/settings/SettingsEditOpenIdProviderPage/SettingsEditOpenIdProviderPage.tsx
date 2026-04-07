import { useMutation, useSuspenseQuery } from '@tanstack/react-query';
import { Link, useRouter } from '@tanstack/react-router';
import { useCallback, useMemo } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { type AddOpenIdProvider, OpenIdProviderKind } from '../../../shared/api/types';
import { EditPage } from '../../../shared/components/EditPage/EditPage';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { getExternalProviderQueryOptions } from '../../../shared/query';
import { joinCsv } from '../../../shared/utils/csv';
import { EditCustomProviderForm } from './form/EditCustomProviderForm';
import { EditGoogleProviderForm } from './form/EditGoogleProviderForm';
import { EditJumpCloudProviderForm } from './form/EditJumpCloudProviderForm';
import { EditMicrosoftProviderForm } from './form/EditMicrosoftProviderForm';
import { EditOktaProviderForm } from './form/EditOktaProviderForm';

const breadcrumbs = [
  <Link to="/settings/openid" key={0}>
    {m.settings_openid_providers_breadcrumb()}
  </Link>,
  <Link to="/settings/edit-openid" key={1}>
    {m.controls_edit()}
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

  const handleDelete = (name: string) => {
    openModal(ModalName.ConfirmAction, {
      title: m.settings_openid_provider_delete_confirm_title(),
      contentMd: m.settings_openid_provider_delete_confirm_body(),
      actionPromise: () => api.openIdProvider.deleteOpenIdProvider(name),
      invalidateKeys: [['settings'], ['info'], ['openid']],
      submitProps: { text: m.controls_delete(), variant: 'critical' },
      onSuccess: () => {
        Snackbar.default(m.settings_openid_provider_delete_success());
        router.history.back();
      },
      onError: () => Snackbar.error(m.settings_openid_provider_delete_failed()),
    });
  };

  const handleSubmit = useCallback(
    async (values: Partial<AddOpenIdProvider>) => {
      if (isPresent(formData)) {
        const normalizedFormData = {
          ...formData,
          directory_sync_group_match: joinCsv(formData.directory_sync_group_match),
        };
        await mutateAsync({ ...normalizedFormData, ...values });
      }
    },
    [formData, mutateAsync],
  );

  if (!formData) return null;

  return (
    <EditPage
      id="edit-openid-provider-page"
      pageTitle={m.settings_page_title()}
      links={breadcrumbs}
      headerProps={{
        title: m.settings_openid_provider_edit_title(),
      }}
    >
      {formData.name === OpenIdProviderKind.Google && (
        <EditGoogleProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => handleDelete(formData.name)}
        />
      )}
      {formData.name === OpenIdProviderKind.Microsoft && (
        <EditMicrosoftProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => handleDelete(formData.name)}
        />
      )}
      {formData.name === OpenIdProviderKind.Okta && (
        <EditOktaProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => handleDelete(formData.name)}
        />
      )}
      {formData.name === OpenIdProviderKind.JumpCloud && (
        <EditJumpCloudProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => handleDelete(formData.name)}
        />
      )}
      {formData.name === OpenIdProviderKind.Custom && (
        <EditCustomProviderForm
          onSubmit={handleSubmit}
          provider={formData}
          onDelete={() => handleDelete(formData.name)}
        />
      )}
    </EditPage>
  );
};
