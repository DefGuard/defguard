import { useMutation, useQuery } from '@tanstack/react-query';
import { Link } from '@tanstack/react-router';
import { useMemo } from 'react';
import z from 'zod';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import type { Settings } from '../../../shared/api/types';
import { Breadcrumbs } from '../../../shared/components/Breadcrumbs/Breadcrumbs';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Page } from '../../../shared/components/Page/Page';
import { SettingsCard } from '../../../shared/components/SettingsCard/SettingsCard';
import { SettingsHeader } from '../../../shared/components/SettingsHeader/SettingsHeader';
import { SettingsLayout } from '../../../shared/components/SettingsLayout/SettingsLayout';
import { createNumericSelectOptions } from '../../../shared/const/numericSelectOptions';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';

const breadcrumbs = [
  <Link
    to="/settings"
    search={{
      tab: 'general',
    }}
    key={0}
  >
    {m.settings_breadcrumb_general()}
  </Link>,
  <Link to="/settings/instance" key={1}>
    {m.settings_breadcrumb_instance()}
  </Link>,
];

export const SettingsInstancePage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_instance_title()}
          subtitle={m.settings_instance_subtitle()}
        />
        {isPresent(settings) && (
          <SettingsCard>
            <Content settings={settings} />
          </SettingsCard>
        )}
      </SettingsLayout>
    </Page>
  );
};

const formSchema = z.object({
  instance_name: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required())
    .min(
      3,
      m.form_error_min_len({
        length: 3,
      }),
    )
    .max(64, m.form_error_max_len({ length: 64 })),
  public_proxy_url: z
    .url(m.initial_setup_general_config_error_public_proxy_url_invalid())
    .min(1, m.initial_setup_general_config_error_public_proxy_url_required()),
  authentication_period_days: z.number().min(1, m.form_error_invalid()),
});

type FormFields = z.infer<typeof formSchema>;

const sessionDurationOptions = createNumericSelectOptions(
  [1, 2, 3, 7, 10, 14, 30],
  (value) =>
    value === 1
      ? m.settings_duration_one_day()
      : m.settings_duration_days({ days: value }),
);

const Content = ({ settings }: { settings: Settings }) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
    onSuccess: () => {
      Snackbar.success(m.settings_msg_saved());
    },
    onError: () => {
      Snackbar.error(m.settings_msg_save_failed());
    },
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      instance_name: settings.instance_name ?? '',
      public_proxy_url: settings.public_proxy_url ?? '',
      authentication_period_days: settings.authentication_period_days ?? 7,
    }),
    [
      settings.instance_name,
      settings.public_proxy_url,
      settings.authentication_period_days,
    ],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value }) => {
      await mutateAsync(value);
      form.reset(value);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <form.AppField name="instance_name">
          {(field) => (
            <field.FormInput required label={m.settings_instance_label_name()} />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="public_proxy_url">
          {(field) => (
            <field.FormInput
              required
              label={m.settings_instance_label_public_proxy_url()}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="authentication_period_days">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_instance_label_session_duration()}
              options={sessionDurationOptions}
            />
          )}
        </form.AppField>
      </form.AppForm>
      <form.Subscribe
        selector={(s) => ({
          isDefault: s.isDefaultValue || s.isPristine,
          isSubmitting: s.isSubmitting,
          canSubmit: s.canSubmit,
        })}
      >
        {({ isDefault, isSubmitting, canSubmit }) => (
          <Controls>
            <div className="right">
              <Button
                variant="primary"
                text={m.controls_save_changes()}
                disabled={isDefault || !canSubmit}
                loading={isSubmitting}
                type="submit"
                onClick={() => {
                  form.handleSubmit();
                }}
              />
            </div>
          </Controls>
        )}
      </form.Subscribe>
    </form>
  );
};
