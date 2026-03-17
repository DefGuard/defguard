import { useMutation } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { isValidDefguardUrl } from '../../../shared/utils/defguardUrl';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardGeneralConfigurationStep = () => {
  const { mutateAsync } = useMutation({
    mutationFn: api.settings.patchSettings,
    meta: {
      invalidate: [['settings'], ['migration', 'state']],
    },
  });

  const formSchema = useMemo(
    () =>
      z.object({
        defguard_url: z
          .url(m.migration_wizard_general_config_error_invalid_url())
          .min(1, m.migration_wizard_general_config_error_defguard_url_required())
          .refine(
            isValidDefguardUrl,
            m.migration_wizard_general_config_error_defguard_url_invalid_host(),
          ),
        public_proxy_url: z
          .url(m.migration_wizard_general_config_error_public_proxy_url_invalid())
          .min(1, m.migration_wizard_general_config_error_public_proxy_url_required()),
      }),
    [],
  );
  type FormFields = z.infer<typeof formSchema>;

  const defaultValues = useMigrationWizardStore(
    useShallow(
      (s): FormFields => ({
        defguard_url: s.defguard_url,
        public_proxy_url: s.public_proxy_url,
      }),
    ),
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
      useMigrationWizardStore.setState({
        defguard_url: value.defguard_url,
        public_proxy_url: value.public_proxy_url,
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
          <DescriptionBlock title="Private URL">
            <p>{`This URL will be used to access and control Defguard. It should not be exposed to the Internet only to the internal or VPN network. You can learn more about our security approach in the video below.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="defguard_url">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_defguard_url()}
                type="text"
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <DescriptionBlock title="Public URL">
            <p>{`This URL will be used to access and control Defguard. It should not be exposed to the Internet only to the internal or VPN network. You can learn more about our security approach in the video below.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="public_proxy_url">
            {(field) => (
              <field.FormInput
                required
                label={m.migration_wizard_general_config_label_public_proxy_url()}
                type="text"
              />
            )}
          </form.AppField>
          <form.Subscribe selector={(s) => s.isSubmitting}>
            {(isSubmitting) => (
              <Controls>
                <Button
                  variant="outlined"
                  text={m.controls_back()}
                  disabled={isSubmitting}
                  onClick={() => {
                    useMigrationWizardStore.getState().back();
                  }}
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
