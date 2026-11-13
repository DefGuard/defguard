import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import {
  OpenIdProviderKind,
  type OpenIdProviderKindValue,
  OpenIdProviderUsernameHandling,
} from '../../../../shared/api/types';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import {
  detectJumpcloudRegion,
  jumpcloudBaseUrls,
  supportedSyncProviders,
} from '../../../../shared/constants';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { mfaAvailabilityInvalidateKey } from '../../../../shared/query';
import {
  ClientAuthMethodField,
  ClientCertificateFields,
  clientAuthMethodFieldNames,
  clientCertificateFieldNames,
} from '../../components/ClientCertificateFields/ClientCertificateFields';
import {
  ClientAuthMethod,
  clientCredentialsPayload,
  clientCredentialsSchema,
  formatMicrosoftBaseUrl,
  jumpcloudRegionOptions,
  providerUsernameHandlingOptions,
  refineClientCertificate,
  validateExternalProviderWizard,
} from '../../consts';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

const baseUrlHidden: Set<OpenIdProviderKindValue> = new Set([
  OpenIdProviderKind.JumpCloud,
  OpenIdProviderKind.Microsoft,
  OpenIdProviderKind.Google,
]);

export const AddExternalOpenIdClientSettingsStep = () => {
  const storeData = useAddExternalOpenIdStore((s) => s.providerState);
  const provider = useAddExternalOpenIdStore((s) => s.provider);
  const next = useAddExternalOpenIdStore((s) => s.next);
  const certificateError = useAddExternalOpenIdStore((s) => s.certificateError);
  const clearCertificateError = () =>
    useAddExternalOpenIdStore.setState({ certificateError: null });

  const { mutateAsync } = useMutation({
    mutationFn: validateExternalProviderWizard,
    onSuccess: (result) => {
      if (result === null) return;
      if (typeof result === 'boolean') {
        useAddExternalOpenIdStore.setState({
          testResult: result,
        });
        next();
      } else {
        useAddExternalOpenIdStore.setState({
          testResult: result.success,
          testMessage: result.message,
        });
        next();
      }
    },
    meta: {
      invalidate: [
        ['settings'],
        ['info'],
        ['openid', 'provider'],
        ['settings_essentials'],
        mfaAvailabilityInvalidateKey,
      ],
    },
  });

  const formSchema = useMemo(
    () =>
      z
        .object({
          base_url: z.url(m.form_error_invalid()).trim().min(1, m.form_error_required()),
          client_id: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required()),
          client_secret: z.string().trim(),
          ...clientCredentialsSchema.shape,
          display_name: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required()),
          create_account: z.boolean(m.form_error_invalid()),
          disable_password_management: z.boolean(),
          username_handling: z.enum(OpenIdProviderUsernameHandling),
          microsoftTenantId: z.string().trim().nullable(),
          jumpcloud_region: z.enum(['us', 'eu', 'in']),
        })
        .superRefine((values, ctx) => {
          if (
            values.client_auth_method === ClientAuthMethod.Secret &&
            values.client_secret.length === 0
          ) {
            ctx.addIssue({
              code: 'custom',
              continue: true,
              message: m.form_error_required(),
              path: ['client_secret'],
            });
          }
          refineClientCertificate(values, ctx);
          if (provider === OpenIdProviderKind.Microsoft) {
            const schema = z
              .string(m.form_error_required())
              .trim()
              .min(1, m.form_error_required());
            const result = schema.safeParse(values.microsoftTenantId);
            if (!result.success) {
              ctx.addIssue({
                code: 'custom',
                continue: true,
                message: result.error.issues[0]?.message ?? m.form_error_required(),
                path: ['microsoftTenantId'],
              });
            }
          }
        }),
    [provider],
  );

  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMemo(
    (): FormFields => ({
      base_url: storeData.base_url,
      client_id: storeData.client_id,
      client_secret: storeData.client_secret ?? '',
      client_auth_method:
        isPresent(storeData.client_certificate) || isPresent(storeData.client_private_key)
          ? ClientAuthMethod.Certificate
          : ClientAuthMethod.Secret,
      client_certificate: storeData.client_certificate ?? null,
      client_private_key: storeData.client_private_key ?? null,
      create_account: storeData.create_account,
      disable_password_management: storeData.disable_password_management,
      display_name: storeData.display_name,
      microsoftTenantId: storeData.microsoftTenantId ?? null,
      username_handling: storeData.username_handling,
      jumpcloud_region: detectJumpcloudRegion(storeData.base_url),
    }),
    [storeData],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      clearCertificateError();
      if (provider === OpenIdProviderKind.JumpCloud) {
        value.base_url = jumpcloudBaseUrls[value.jumpcloud_region];
      }
      const data = clientCredentialsPayload(value);
      if (supportedSyncProviders.has(provider)) {
        next(data);
      } else {
        const storeState = useAddExternalOpenIdStore.getState().providerState;
        await mutateAsync({ ...storeState, ...data });
      }
    },
  });

  return (
    <WizardCard id="add-external-openid-client-step">
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <form.AppField name="display_name">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_openid_provider_label_display_name()}
                helper={m.settings_openid_provider_helper_display_name()}
              />
            )}
          </form.AppField>
          {provider === OpenIdProviderKind.Microsoft && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <form.AppField
                name="microsoftTenantId"
                listeners={{
                  onChange: ({ fieldApi, value }) => {
                    fieldApi.form.setFieldValue(
                      'base_url',
                      formatMicrosoftBaseUrl(value ?? ''),
                    );
                  },
                }}
              >
                {(field) => (
                  <field.FormInput
                    required
                    label={m.settings_openid_provider_label_microsoft_tenant_id()}
                    helper={m.settings_openid_provider_helper_microsoft_tenant_id()}
                  />
                )}
              </form.AppField>
            </>
          )}
          {!baseUrlHidden.has(provider) && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <form.AppField name="base_url">
                {(field) => (
                  <field.FormInput
                    required
                    label={m.settings_openid_provider_label_base_url()}
                    helper={m.settings_openid_provider_helper_base_url()}
                  />
                )}
              </form.AppField>
            </>
          )}
          {provider === OpenIdProviderKind.Microsoft && (
            <>
              <SizedBox height={ThemeSpacing.Xl3} />
              <ClientAuthMethodField form={form} fields={clientAuthMethodFieldNames} />
            </>
          )}
          <SizedBox
            height={
              provider === OpenIdProviderKind.Microsoft
                ? ThemeSpacing.Xl
                : ThemeSpacing.Xl2
            }
          />
          <form.AppField name="client_id">
            {(field) => (
              <field.FormInput
                required
                label={m.settings_openid_provider_label_client_id()}
                helper={m.settings_openid_provider_helper_client_id()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.Subscribe selector={(s) => s.values.client_auth_method}>
            {(authMethod) =>
              authMethod === ClientAuthMethod.Secret ? (
                <form.AppField name="client_secret">
                  {(field) => (
                    <field.FormInput
                      required
                      label={m.settings_openid_provider_label_client_secret()}
                      type="password"
                      helper={m.settings_openid_provider_helper_client_secret()}
                    />
                  )}
                </form.AppField>
              ) : (
                <ClientCertificateFields
                  form={form}
                  fields={clientCertificateFieldNames}
                  error={certificateError}
                  onChange={clearCertificateError}
                />
              )
            }
          </form.Subscribe>
          <SizedBox
            height={
              provider === OpenIdProviderKind.Microsoft
                ? ThemeSpacing.Xl3
                : ThemeSpacing.Xl2
            }
          />
          <form.AppField name="username_handling">
            {(field) => (
              <field.FormSelect
                options={providerUsernameHandlingOptions}
                required
                label={m.settings_openid_provider_label_username_handling()}
                helper={m.settings_openid_provider_helper_username_handling()}
              />
            )}
          </form.AppField>
          {provider === OpenIdProviderKind.JumpCloud && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <form.AppField name="jumpcloud_region">
                {(field) => (
                  <field.FormSelect
                    options={jumpcloudRegionOptions}
                    label={m.settings_openid_provider_label_jumpcloud_region()}
                    helper={m.settings_openid_provider_helper_jumpcloud_region()}
                  />
                )}
              </form.AppField>
            </>
          )}
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="create_account">
            {(field) => (
              <field.FormInteractiveBlock
                variant="checkbox"
                title={m.settings_openid_provider_create_account_title()}
                content={m.settings_openid_provider_create_account_content()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="disable_password_management">
            {(field) => (
              <field.FormInteractiveBlock
                variant="checkbox"
                title={m.settings_openid_provider_disable_password_management()}
                content={m.settings_openid_provider_disable_password_management_content()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.Subscribe
            selector={(s) => ({
              isSubmitting: s.isSubmitting,
            })}
          >
            {({ isSubmitting }) => (
              <Controls>
                <div className="right">
                  <Button
                    variant="primary"
                    text={m.controls_continue()}
                    testId="continue"
                    loading={isSubmitting}
                    onClick={() => {
                      form.handleSubmit();
                    }}
                  />
                </div>
              </Controls>
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
