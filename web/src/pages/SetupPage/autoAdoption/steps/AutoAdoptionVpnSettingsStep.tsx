import { useMutation } from '@tanstack/react-query';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { useAppForm } from '../../../../shared/form';
import { formChangeLogic } from '../../../../shared/formLogic';
import { Validate } from '../../../../shared/validate';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

const formSchema = z.object({
  vpn_public_ip: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .refine(
      (value) =>
        Validate.any(value, [
          Validate.IPv4,
          Validate.IPv6,
          Validate.Domain,
          Validate.Hostname,
        ]),
      m.initial_setup_auto_adoption_vpn_error_invalid_value(),
    ),
  vpn_wireguard_port: z
    .number()
    .min(1, m.form_error_required())
    .max(65535, m.initial_setup_auto_adoption_vpn_error_port_too_large()),
  vpn_gateway_address: z
    .string()
    .trim()
    .min(1, m.form_error_required())
    .refine(
      (value) => Validate.any(value, [Validate.CIDRv4, Validate.CIDRv6], true),
      m.initial_setup_auto_adoption_vpn_error_invalid_value(),
    ),
  vpn_allowed_ips: z
    .string()
    .trim()
    .nullable()
    .refine((val) => {
      if (!val) return true;
      return Validate.any(
        val,
        [
          Validate.IPv4,
          Validate.IPv6,
          (v) => Validate.CIDRv4(v, true),
          (v) => Validate.CIDRv6(v, true),
        ],
        true,
      );
    }, m.form_error_invalid()),
  vpn_dns_server_ip: z
    .string()
    .trim()
    .nullable()
    .refine((val) => {
      if (!val) return true;
      return Validate.any(
        val,
        [Validate.IPv4, Validate.IPv6, Validate.Domain, Validate.Hostname],
        true,
      );
    }),
});

type FormFields = z.infer<typeof formSchema>;

export const AutoAdoptionVpnSettingsStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);

  const { mutate: setVpnSettings, isPending } = useMutation({
    mutationFn: api.initial_setup.setAutoAdoptionVpnSettings,
    onSuccess: () => {
      setActiveStep(AutoAdoptionSetupStep.MfaSetup);
    },
  });
  const defaultValues = useAutoAdoptionSetupWizardStore(
    useShallow(
      (s): FormFields => ({
        vpn_public_ip: s.vpn_public_ip,
        vpn_wireguard_port: s.vpn_wireguard_port,
        vpn_gateway_address: s.vpn_gateway_address,
        vpn_allowed_ips: s.vpn_allowed_ips,
        vpn_dns_server_ip: s.vpn_dns_server_ip,
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
    onSubmit: ({ value }) => {
      const storeValue = {
        ...value,
        vpn_allowed_ips: value.vpn_allowed_ips ?? '',
        vpn_dns_server_ip: value.vpn_dns_server_ip ?? '',
      };
      useAutoAdoptionSetupWizardStore.setState(storeValue);
      setVpnSettings(storeValue);
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
          <p>{m.initial_setup_auto_adoption_vpn_intro()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <div className="vpn-top-row">
            <form.AppField name="vpn_public_ip">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_auto_adoption_vpn_label_public_ip()}
                />
              )}
            </form.AppField>
            <form.AppField name="vpn_wireguard_port">
              {(field) => (
                <field.FormInput
                  required
                  label={m.initial_setup_auto_adoption_vpn_label_wireguard_port()}
                  type="number"
                />
              )}
            </form.AppField>
          </div>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.initial_setup_auto_adoption_vpn_gateway_description()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_gateway_address">
            {(field) => (
              <field.FormInput
                required
                label={m.initial_setup_auto_adoption_vpn_label_gateway_address()}
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.initial_setup_auto_adoption_vpn_allowed_ips_description()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_allowed_ips">
            {(field) => (
              <field.FormInput
                label={m.initial_setup_auto_adoption_vpn_label_allowed_ips()}
              />
            )}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>{m.initial_setup_auto_adoption_vpn_dns_description()}</p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_dns_server_ip">
            {(field) => (
              <field.FormInput
                label={m.initial_setup_auto_adoption_vpn_label_dns_server_ip()}
              />
            )}
          </form.AppField>
        </form.AppForm>
      </form>
      <SizedBox height={ThemeSpacing.Xl3} />
      <Divider />
      <Controls>
        <Button
          variant="outlined"
          text={m.initial_setup_controls_back()}
          onClick={() => {
            useAutoAdoptionSetupWizardStore.setState({
              ...form.state.values,
              vpn_allowed_ips: form.state.values.vpn_allowed_ips ?? '',
              vpn_dns_server_ip: form.state.values.vpn_dns_server_ip ?? '',
            });
            setActiveStep(AutoAdoptionSetupStep.ExternalUrlSslConfig);
          }}
        />
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
