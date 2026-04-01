import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { EvenSplit } from '../../../shared/defguard-ui/components/EvenSplit/EvenSplit';
import type { SelectOption } from '../../../shared/defguard-ui/components/Select/types';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

type ValidityValue = 1 | 2 | 3 | 5 | 10;

const validityOptions: SelectOption<ValidityValue>[] = [
  { key: 1, label: m.migration_wizard_ca_validity_one_year(), value: 1 },
  { key: 2, label: m.migration_wizard_ca_validity_years({ years: 2 }), value: 2 },
  { key: 3, label: m.migration_wizard_ca_validity_years({ years: 3 }), value: 3 },
  { key: 5, label: m.migration_wizard_ca_validity_years({ years: 5 }), value: 5 },
  {
    key: 10,
    label: m.migration_wizard_ca_validity_years({ years: 10 }),
    value: 10,
  },
];

type CreateCAFormFields = CreateCAStoreValues;

type CreateCAStoreValues = {
  ca_common_name: string;
  ca_email: string;
  ca_validity_period_years: number;
};

export const MigrationWizardCAStep = () => {
  const createCAdefaultValues = useMigrationWizardStore(
    useShallow(
      (s): CreateCAFormFields => ({
        ca_common_name: s.ca_common_name,
        ca_email: s.ca_email,
        ca_validity_period_years: s.ca_validity_period_years,
      }),
    ),
  );

  const createFormSchema = useMemo(
    () =>
      z.object({
        ca_common_name: z
          .string()
          .min(1, m.migration_wizard_ca_error_common_name_required()),
        ca_email: z
          .email(m.migration_wizard_ca_error_email_invalid())
          .min(1, m.migration_wizard_ca_error_email_required()),
        ca_validity_period_years: z
          .number()
          .min(1, m.migration_wizard_ca_error_validity_min()),
      }),
    [],
  );

  const { mutateAsync: createCA } = useMutation({
    mutationFn: api.migration.ca.createCA,
    onError: (error) => {
      console.error('Failed to create CA:', error);
      Snackbar.error(m.migration_wizard_ca_error_create_failed());
    },
    meta: {
      invalidate: ['migration', 'ca'],
    },
  });

  const form = useAppForm({
    defaultValues: createCAdefaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: createFormSchema,
      onChange: createFormSchema,
    },
    onSubmit: async ({ value }) => {
      useMigrationWizardStore.setState({
        ca_common_name: value.ca_common_name,
        ca_email: value.ca_email,
        ca_validity_period_years: value.ca_validity_period_years,
      });
      await createCA({
        common_name: value.ca_common_name,
        email: value.ca_email,
        validity_period_years: value.ca_validity_period_years,
      });
      useMigrationWizardStore.getState().next();
    },
  });

  return (
    <WizardCard>
      <form
        onSubmit={(e) => {
          e.stopPropagation();
          e.preventDefault();
          form.handleSubmit();
        }}
      >
        <form.AppForm>
          <EvenSplit>
            <form.AppField name="ca_common_name">
              {(field) => (
                <field.FormInput
                  required
                  label={m.migration_wizard_ca_label_common_name()}
                  helper={m.migration_wizard_ca_helper_common_name()}
                  type="text"
                  placeholder={m.migration_wizard_ca_placeholder_common_name()}
                />
              )}
            </form.AppField>
            <form.AppField name="ca_email">
              {(field) => (
                <field.FormInput
                  required
                  label={m.migration_wizard_ca_label_email()}
                  helper={m.migration_wizard_ca_helper_email()}
                  placeholder={m.migration_wizard_ca_placeholder_email()}
                />
              )}
            </form.AppField>
          </EvenSplit>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="ca_validity_period_years">
            {(field) => (
              <field.FormSelect
                required
                label={m.migration_wizard_ca_label_validity()}
                helper={m.migration_wizard_ca_helper_validity()}
                options={validityOptions}
              />
            )}
          </form.AppField>
          <form.Subscribe
            selector={(s) => ({
              isSubmitting: s.isSubmitting,
            })}
          >
            {({ isSubmitting }) => (
              <Controls>
                <Button
                  variant="outlined"
                  text={m.controls_back()}
                  onClick={() => {
                    useMigrationWizardStore.getState().back();
                  }}
                  disabled={isSubmitting}
                />
                <div className="right">
                  <Button
                    text={m.controls_continue()}
                    type="submit"
                    loading={isSubmitting}
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
