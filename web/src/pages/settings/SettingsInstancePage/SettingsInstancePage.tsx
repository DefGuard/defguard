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
  <Link to="/settings/instance" key={1}>
    Instance settings
  </Link>,
];

export const SettingsInstancePage = () => {
  const { data: settings } = useQuery(getSettingsQueryOptions);
  return (
    <Page title="Settings">
      <Breadcrumbs links={breadcrumbs} />
      <SettingsLayout>
        <SettingsHeader
          icon="customize"
          title="Instance settings"
          subtitle="Here you can configure general instance parameters."
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
});

type FormFields = z.infer<typeof formSchema>;

const Content = ({ settings }: { settings: Settings }) => {
  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: ['settings'],
    },
  });

  const defaultValues = useMemo(
    (): FormFields => ({
      instance_name: settings.instance_name ?? '',
    }),
    [settings.instance_name],
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
          {(field) => <field.FormInput required label="Instance name" />}
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
