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
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { MarkedSection } from '../../../shared/defguard-ui/components/MarkedSection/MarkedSection';
import { MarkedSectionHeader } from '../../../shared/defguard-ui/components/MarkedSectionHeader/MarkedSectionHeader';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getSettingsQueryOptions } from '../../../shared/query';
import {
  createNumericSelectOptions,
  withNumericFallbackOption,
} from '../../../shared/utils/numericSelectOptions';

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
  disable_stats_purge: z.boolean(),
  stats_purge_frequency_hours: z.number(m.form_error_required()).int().min(1),
  stats_purge_threshold_days: z.number(m.form_error_required()).int().min(1),
});

type FormFields = z.infer<typeof formSchema>;

const sessionDurationOptions = createNumericSelectOptions({
  1: m.settings_duration_one_day(),
  2: m.settings_duration_days({ days: 2 }),
  3: m.settings_duration_days({ days: 3 }),
  7: m.settings_duration_days({ days: 7 }),
  10: m.settings_duration_days({ days: 10 }),
  14: m.settings_duration_days({ days: 14 }),
  30: m.settings_duration_days({ days: 30 }),
});

const sessionDurationFallbackUnit = 'days';

const statsPurgeFrequencyOptions = createNumericSelectOptions({
  1: m.settings_duration_one_hour(),
  12: m.settings_duration_hours({ hours: 12 }),
  24: m.settings_duration_one_day(),
  48: m.settings_duration_days({ days: 2 }),
  168: m.settings_duration_one_week(),
  720: m.settings_duration_one_month(),
});

const statsPurgeThresholdOptions = createNumericSelectOptions({
  1: m.settings_duration_one_day(),
  7: m.settings_duration_days({ days: 7 }),
  14: m.settings_duration_days({ days: 14 }),
  30: m.settings_duration_days({ days: 30 }),
  90: m.settings_duration_days({ days: 90 }),
});

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
      disable_stats_purge: settings.disable_stats_purge ?? false,
      stats_purge_frequency_hours: settings.stats_purge_frequency_hours ?? 24,
      stats_purge_threshold_days: settings.stats_purge_threshold_days ?? 30,
    }),
    [
      settings.instance_name,
      settings.public_proxy_url,
      settings.authentication_period_days,
      settings.disable_stats_purge,
      settings.stats_purge_frequency_hours,
      settings.stats_purge_threshold_days,
    ],
  );

  const sessionDurationSelectOptions = useMemo(
    () =>
      withNumericFallbackOption(
        sessionDurationOptions,
        defaultValues.authentication_period_days,
        sessionDurationFallbackUnit,
      ),
    [defaultValues.authentication_period_days],
  );

  const statsPurgeFrequencySelectOptions = useMemo(
    () =>
      withNumericFallbackOption(
        statsPurgeFrequencyOptions,
        defaultValues.stats_purge_frequency_hours,
        'hours',
      ),
    [defaultValues.stats_purge_frequency_hours],
  );

  const statsPurgeThresholdSelectOptions = useMemo(
    () =>
      withNumericFallbackOption(
        statsPurgeThresholdOptions,
        defaultValues.stats_purge_threshold_days,
        'days',
      ),
    [defaultValues.stats_purge_threshold_days],
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
        <MarkedSection icon="settings">
          <MarkedSectionHeader
            title={m.settings_instance_section_core()}
            description={m.settings_instance_section_core_description()}
          />
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
                options={sessionDurationSelectOptions}
              />
            )}
          </form.AppField>
        </MarkedSection>
        <Divider spacing={ThemeSpacing.Xl2} />
        <MarkedSection icon="activity">
          <MarkedSectionHeader
            title={m.settings_instance_section_data_retention()}
            description={m.settings_instance_section_data_retention_description()}
          />
          <form.AppField name="disable_stats_purge">
            {(field) => (
              <field.FormInteractiveBlock
                variant="toggle"
                title={m.settings_vpn_stats_toggle_disable_title()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="stats_purge_frequency_hours">
            {(field) => (
              <field.FormSelect
                required
                label={m.settings_vpn_stats_label_purge_frequency()}
                options={statsPurgeFrequencySelectOptions}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="stats_purge_threshold_days">
            {(field) => (
              <field.FormSelect
                required
                label={m.settings_vpn_stats_label_purge_threshold()}
                options={statsPurgeThresholdSelectOptions}
              />
            )}
          </form.AppField>
        </MarkedSection>
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
