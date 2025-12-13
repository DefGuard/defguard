import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../../paraglide/messages';
import {
  OpenIdProviderUsernameHandling,
  type OpenIdProviderUsernameHandlingValue,
} from '../../../../shared/api/types';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import type { SelectOption } from '../../../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import {
  ExternalProvider,
  type ExternalProviderValue,
} from '../../../settings/shared/types';
import { useAddExternalOpenIdStore } from '../../useAddExternalOpenIdStore';

const userHandlingOptions: SelectOption<OpenIdProviderUsernameHandlingValue>[] = [
  {
    key: OpenIdProviderUsernameHandling.RemoveForbidden,
    label: 'Remove forbidden characters',
    value: OpenIdProviderUsernameHandling.RemoveForbidden,
  },
  {
    key: OpenIdProviderUsernameHandling.ReplaceForbidden,
    label: 'Replace forbidden characters',
    value: OpenIdProviderUsernameHandling.ReplaceForbidden,
  },
  {
    key: OpenIdProviderUsernameHandling.PruneEmailDomain,
    label: 'Prune email domain',
    value: OpenIdProviderUsernameHandling.PruneEmailDomain,
  },
];

const baseUrlHidden: Set<ExternalProviderValue> = new Set([
  ExternalProvider.JumpCloud,
  ExternalProvider.Microsoft,
  ExternalProvider.Google,
]);

const formatMicrosoftBaseUrl = (tenantId: string) =>
  `https://login.microsoftonline.com/${tenantId}/v2.0`;

export const AddExternalOpenIdClientSettingsStep = () => {
  const storeData = useAddExternalOpenIdStore((s) => s.providerState);
  const provider = useAddExternalOpenIdStore((s) => s.provider);
  const nextStep = useAddExternalOpenIdStore((s) => s.next);

  const formSchema = useMemo(
    () =>
      z
        .object({
          name: z.string(),
          base_url: z.url(m.form_error_invalid()).trim().min(1, m.form_error_required()),
          client_id: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required()),
          client_secret: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required()),
          display_name: z
            .string(m.form_error_required())
            .trim()
            .min(1, m.form_error_required()),
          create_account: z.boolean(m.form_error_invalid()),
          username_handling: z.enum(OpenIdProviderUsernameHandling),
          microsoftTenantId: z.string().trim().nullable(),
        })
        .superRefine((values, ctx) => {
          if (provider === ExternalProvider.Microsoft) {
            const schema = z
              .string(m.form_error_required())
              .trim()
              .min(1, m.form_error_required());
            const result = schema.safeParse(values.microsoftTenantId);
            if (!result.success) {
              ctx.addIssue({
                code: 'custom',
                continue: true,
                message: result.error.message,
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
      name: storeData.name,
      base_url: storeData.base_url,
      client_id: storeData.client_id,
      client_secret: storeData.client_secret,
      create_account: storeData.create_account,
      display_name: storeData.display_name,
      microsoftTenantId: storeData.microsoftTenantId ?? null,
      username_handling: storeData.username_handling,
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
    onSubmit: ({ value }) => {
      nextStep(value);
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
            {(field) => <field.FormInput required label="Display Name" />}
          </form.AppField>
          {provider === ExternalProvider.Microsoft && (
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
                {(field) => <field.FormInput required label="Tenant ID" />}
              </form.AppField>
            </>
          )}
          {!baseUrlHidden.has(provider) && (
            <>
              <SizedBox height={ThemeSpacing.Xl2} />
              <form.AppField name="base_url">
                {(field) => <field.FormInput required label="Base URL" />}
              </form.AppField>
            </>
          )}
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_id">
            {(field) => <field.FormInput required label="Client ID" />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="client_secret">
            {(field) => (
              <field.FormInput required label="Client Secret" type="password" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="username_handling">
            {(field) => (
              <field.FormSelect
                options={userHandlingOptions}
                required
                label="Username handling"
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl2} />
          <form.AppField name="create_account">
            {(field) => (
              <field.FormInteractiveBlock
                variant="checkbox"
                title="Automatically create user account when logging in for the first time through external OpenID."
                content="If this option is enabled, Defguard automatically creates new accounts for users who log in for the first time using an external OpenID. Otherwise, the user account must first be created by an administrator."
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
