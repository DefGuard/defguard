import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isValidDefguardUrl } from '../../../../shared/utils/defguardUrl';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

type FormFields = {
  defguard_url: string;
  public_proxy_url: string;
};

export const AutoAdoptionUrlSettingsStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const defaultValues = useAutoAdoptionSetupWizardStore(
    useShallow(
      (s): FormFields => ({
        defguard_url: s.defguard_url,
        public_proxy_url: s.public_proxy_url,
      }),
    ),
  );

  const formSchema = useMemo(
    () =>
      z.object({
        defguard_url: z
          .url(m.initial_setup_general_config_error_invalid_url())
          .refine(
            isValidDefguardUrl,
            m.initial_setup_general_config_error_defguard_url_invalid_host(),
          )
          .min(1, m.initial_setup_general_config_error_defguard_url_required()),
        public_proxy_url: z
          .url(m.initial_setup_general_config_error_public_proxy_url_invalid())
          .min(1, m.initial_setup_general_config_error_public_proxy_url_required()),
      }),
    [],
  );

  const { mutate, isPending } = useMutation({
    mutationFn: api.initial_setup.setAutoAdoptionUrlSettings,
    meta: {
      invalidate: ['setupStatus'],
    },
    onSuccess: () => {
      setActiveStep(AutoAdoptionSetupStep.VpnSettings);
    },
    onError: (error) => {
      Snackbar.error(m.initial_setup_general_config_error_save_failed());
      console.error('Failed to save URL settings:', error);
    },
  });

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: ({ value }) => {
      useAutoAdoptionSetupWizardStore.setState({
        defguard_url: value.defguard_url,
        public_proxy_url: value.public_proxy_url,
      });

      mutate({
        defguard_url: value.defguard_url,
        public_proxy_url: value.public_proxy_url,
      });
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
          <p>{m.initial_setup_auto_adoption_url_settings_defguard_description()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="defguard_url">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_defguard_url()}
                type="text"
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.initial_setup_auto_adoption_url_settings_public_proxy_description()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="public_proxy_url">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_general_config_label_public_proxy_url()}
                type="text"
              />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <Controls>
        <div className="right">
          <Button
            text={m.initial_setup_controls_continue()}
            onClick={form.handleSubmit}
            loading={isPending}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
