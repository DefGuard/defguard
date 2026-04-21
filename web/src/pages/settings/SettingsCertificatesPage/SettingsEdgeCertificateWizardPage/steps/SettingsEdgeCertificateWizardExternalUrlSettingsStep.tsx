import { useMutation } from '@tanstack/react-query';
import z from 'zod';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import type { ExternalSslType } from '../../../../../shared/api/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../../shared/defguard-ui/components/Divider/Divider';
import { Helper } from '../../../../../shared/defguard-ui/components/Helper/Helper';
import { Radio } from '../../../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import '../../../../SetupPage/autoAdoption/steps/style.scss';
import { getApiErrorMessage } from '../../utils';
import { SettingsEdgeCertificateWizardStep } from '../types';
import { useSettingsEdgeCertificateWizardStore } from '../useSettingsEdgeCertificateWizardStore';

export const SettingsEdgeCertificateWizardExternalUrlSettingsStep = () => {
  const storedSslType = useSettingsEdgeCertificateWizardStore((s) => s.external_ssl_type);

  const formSchema = z.object({
    ssl_type: z.custom<ExternalSslType>(),
    cert_pem_file: z.custom<File | null>().nullable(),
    key_pem_file: z.custom<File | null>().nullable(),
  });

  const { mutate, isPending } = useMutation({
    mutationFn: api.edge.setExternalUrlSettings,
    meta: {
      invalidate: [
        ['core', 'cert', 'certs'],
        ['core', 'cert', 'ca'],
      ],
    },
    onSuccess: (response) => {
      useSettingsEdgeCertificateWizardStore.setState({
        external_ssl_type: form.getFieldValue('ssl_type'),
        external_ssl_cert_info: response.data.cert_info ?? null,
        activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSslConfig,
      });
    },
    onError: (error) => {
      Snackbar.error(getApiErrorMessage(error) ?? m.settings_msg_save_failed());
      console.error('Failed to save edge external URL settings:', error);
    },
  });

  const form = useAppForm({
    defaultValues: {
      ssl_type: (storedSslType ?? 'none') as ExternalSslType,
      cert_pem_file: null as File | null,
      key_pem_file: null as File | null,
    },
    validationLogic: formChangeLogic,
    validators: { onSubmit: formSchema, onChange: formSchema },
    onSubmit: async ({ value }) => {
      if (
        value.ssl_type === 'own_cert' &&
        (!value.cert_pem_file || !value.key_pem_file)
      ) {
        Snackbar.error(
          m.initial_setup_auto_adoption_external_url_settings_upload_files_required(),
        );
        return;
      }

      mutate({
        ssl_type: value.ssl_type,
        cert_pem: value.cert_pem_file ? await value.cert_pem_file.text() : undefined,
        key_pem: value.key_pem_file ? await value.key_pem_file.text() : undefined,
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
          <p>{m.initial_setup_auto_adoption_external_url_settings_description()}</p>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.Subscribe selector={(s) => s.values.ssl_type}>
            {(sslType) => (
              <div className="ssl-options">
                <div className="ssl-option-row">
                  <Radio
                    text={m.initial_setup_auto_adoption_external_url_settings_ssl_option_none()}
                    active={sslType === 'none'}
                    onClick={() => form.setFieldValue('ssl_type', 'none')}
                  />
                  <Helper>
                    {m.initial_setup_auto_adoption_external_url_settings_ssl_option_none_help()}
                  </Helper>
                </div>
                <SizedBox height={ThemeSpacing.Md} />
                <div className="ssl-option-row">
                  <Radio
                    text={m.initial_setup_auto_adoption_external_url_settings_ssl_option_lets_encrypt()}
                    active={sslType === 'lets_encrypt'}
                    onClick={() => form.setFieldValue('ssl_type', 'lets_encrypt')}
                  />
                  <Helper>
                    {m.initial_setup_auto_adoption_external_url_settings_ssl_option_lets_encrypt_help()}
                  </Helper>
                </div>
                <SizedBox height={ThemeSpacing.Md} />
                <div className="ssl-option-row">
                  <Radio
                    text={m.initial_setup_auto_adoption_external_url_settings_ssl_option_defguard_ca()}
                    active={sslType === 'defguard_ca'}
                    onClick={() => form.setFieldValue('ssl_type', 'defguard_ca')}
                  />
                  <Helper>
                    {m.initial_setup_auto_adoption_external_url_settings_ssl_option_defguard_ca_help()}
                  </Helper>
                </div>
                <SizedBox height={ThemeSpacing.Md} />
                <div className="ssl-option-row">
                  <Radio
                    text={m.initial_setup_auto_adoption_external_url_settings_ssl_option_own_cert()}
                    active={sslType === 'own_cert'}
                    onClick={() => form.setFieldValue('ssl_type', 'own_cert')}
                  />
                  <Helper>
                    {m.initial_setup_auto_adoption_external_url_settings_ssl_option_own_cert_help()}
                  </Helper>
                </div>
                {sslType === 'own_cert' && (
                  <div className="cert-upload-section">
                    <SizedBox height={ThemeSpacing.Xl} />
                    <form.AppField name="cert_pem_file">
                      {(field) => (
                        <field.FormUploadField
                          acceptedExtensions={['.pem', '.crt', '.cer']}
                          title={m.initial_setup_auto_adoption_external_url_settings_upload_cert_button()}
                        />
                      )}
                    </form.AppField>
                    <SizedBox height={ThemeSpacing.Lg} />
                    <form.AppField name="key_pem_file">
                      {(field) => (
                        <field.FormUploadField
                          acceptedExtensions={['.pem', '.key']}
                          title={m.initial_setup_auto_adoption_external_url_settings_upload_key_button()}
                        />
                      )}
                    </form.AppField>
                  </div>
                )}
              </div>
            )}
          </form.Subscribe>
        </form.AppForm>
      </form>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Divider />
      <Controls>
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={form.handleSubmit}
            loading={isPending}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
