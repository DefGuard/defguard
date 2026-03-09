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
import {
  createNumericSelectOptions,
  withNumericFallbackOption,
} from '../../../shared/const/numericSelectOptions';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
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
  <Link to="/settings/vpn-stats" key={1}>
    {m.settings_breadcrumb_vpn_stats()}
  </Link>,
];

export const SettingsVpnStatsPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title={m.settings_page_title()}>
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title={m.settings_vpn_stats_title()}
          subtitle={m.settings_vpn_stats_subtitle()}
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
  disable_stats_purge: z.boolean(),
  stats_purge_frequency_hours: z.number(m.form_error_required()).int().min(1),
  stats_purge_threshold_days: z.number(m.form_error_required()).int().min(1),
});

type FormFields = z.infer<typeof formSchema>;

const formatStatsPurgeFrequencyLabel = (value: number) => {
  switch (value) {
    case 24:
      return m.settings_duration_one_day();
    case 48:
      return m.settings_duration_days({ days: 2 });
    case 168:
      return m.settings_duration_one_week();
    case 720:
      return m.settings_duration_one_month();
    case 1:
      return m.settings_duration_one_hour();
    default:
      return m.settings_duration_hours({ hours: value });
  }
};

const statsPurgeFrequencyBaseOptions = createNumericSelectOptions(
  [1, 12, 24, 48, 168, 720],
  formatStatsPurgeFrequencyLabel,
);

const statsPurgeThresholdBaseOptions = createNumericSelectOptions(
  [1, 7, 14, 30, 90],
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
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      disable_stats_purge: settings.disable_stats_purge ?? false,
      stats_purge_frequency_hours: settings.stats_purge_frequency_hours ?? 24,
      stats_purge_threshold_days: settings.stats_purge_threshold_days ?? 30,
    }),
    [settings],
  );

  const statsPurgeFrequencyOptions = useMemo(
    () =>
      withNumericFallbackOption(
        statsPurgeFrequencyBaseOptions,
        defaultValues.stats_purge_frequency_hours,
        formatStatsPurgeFrequencyLabel,
      ),
    [defaultValues.stats_purge_frequency_hours],
  );

  const statsPurgeThresholdOptions = useMemo(
    () =>
      withNumericFallbackOption(
        statsPurgeThresholdBaseOptions,
        defaultValues.stats_purge_threshold_days,
        (value) =>
          value === 1
            ? m.settings_duration_one_day()
            : m.settings_duration_days({ days: value }),
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
              options={statsPurgeFrequencyOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="stats_purge_threshold_days">
          {(field) => (
            <field.FormSelect
              required
              label={m.settings_vpn_stats_label_purge_threshold()}
              options={statsPurgeThresholdOptions}
            />
          )}
        </form.AppField>
      </form.AppForm>
      <form.Subscribe
        selector={(s) => ({
          isDefault: s.isDefaultValue || s.isPristine,
          isSubmitting: s.isSubmitting,
        })}
      >
        {({ isDefault, isSubmitting }) => (
          <Controls>
            <div className="right">
              <Button
                variant="primary"
                text={m.controls_save_changes()}
                disabled={isDefault}
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
