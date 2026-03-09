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
  formatDaySelectLabel,
  formatHourSelectLabel,
  type NumericSelectOption,
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
    General
  </Link>,
  <Link to="/settings/vpn-stats" key={1}>
    VPN stats
  </Link>,
];

export const SettingsVpnStatsPage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title="VPN stats"
          subtitle="Configure statistics purge behavior for VPN data."
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

const statsPurgeFrequencyBaseOptions: NumericSelectOption[] = [
  { key: 1, value: 1, label: '1h' },
  { key: 12, value: 12, label: '12h' },
  { key: 24, value: 24, label: '1 day' },
  { key: 48, value: 48, label: '2 days' },
  { key: 168, value: 168, label: '1 week' },
  { key: 720, value: 720, label: '1 month' },
];

const statsPurgeThresholdBaseOptions = createNumericSelectOptions(
  [1, 7, 14, 30, 90],
  formatDaySelectLabel,
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
        formatHourSelectLabel,
      ),
    [defaultValues.stats_purge_frequency_hours],
  );

  const statsPurgeThresholdOptions = useMemo(
    () =>
      withNumericFallbackOption(
        statsPurgeThresholdBaseOptions,
        defaultValues.stats_purge_threshold_days,
        formatDaySelectLabel,
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
              title="Disable stats purge"
              content="Disables automatic statistics cleanup task."
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="stats_purge_frequency_hours">
          {(field) => (
            <field.FormSelect
              required
              label="Stats purge frequency"
              options={statsPurgeFrequencyOptions}
            />
          )}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="stats_purge_threshold_days">
          {(field) => (
            <field.FormSelect
              required
              label="Stats purge threshold"
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
