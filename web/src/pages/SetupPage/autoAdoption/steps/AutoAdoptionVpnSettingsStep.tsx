import { useMutation } from '@tanstack/react-query';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import api from '../../../../shared/api/api';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ModalControls } from '../../../../shared/defguard-ui/components/ModalControls/ModalControls';
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
    .min(1, 'Required')
    .refine(
      (value) => Validate.any(value, [Validate.IPv4, Validate.IPv6, Validate.Domain]),
      'Invalid value',
    ),
  vpn_wireguard_port: z.number().min(1, 'Required').max(65535, 'Port is too large'),
  vpn_gateway_address: z
    .string()
    .trim()
    .min(1, 'Required')
    .refine(
      (value) => Validate.any(value, [Validate.CIDRv4, Validate.CIDRv6], true),
      'Invalid value',
    ),
  vpn_allowed_ips: z.string().trim(),
  vpn_dns_server_ip: z.string().trim(),
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
      useAutoAdoptionSetupWizardStore.setState(value);
      setVpnSettings(value);
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
          <p>
            To make the VPN operational, a few basic parameters must be configured.
            WireGuard® needs to be publicly accessible on a specific IP address and UDP
            port. This IP does not have to be set directly on the gateway it can be
            configured on your firewall or router and forwarded to the Defguard Gateway.
          </p>
          <SizedBox height={ThemeSpacing.Lg} />
          <div className="vpn-top-row">
            <form.AppField name="vpn_public_ip">
              {(field) => <field.FormInput required label="Public IP" />}
            </form.AppField>
            <form.AppField name="vpn_wireguard_port">
              {(field) => (
                <field.FormInput required label="WireGuard Port" type="number" />
              )}
            </form.AppField>
          </div>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>
            Please provide the internal VPN network IP address for the Defguard Gateway.
            The VPN network will be derived from this address (e.g., 10.10.10.1 →
            10.10.10.0). You may specify multiple addresses separated by commas; the first
            will be used as the primary address for device IP assignment.
          </p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_gateway_address">
            {(field) => <field.FormInput required label="Gateway Address" />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>
            If you want your local networks to be accessible from VPN, list them in
            addresses/masks format below:
          </p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_allowed_ips">
            {(field) => <field.FormInput label="Allowed IPs" />}
          </form.AppField>
          <Divider spacing={ThemeSpacing.Xl} />
          <p>
            Configure (optionally) a custom DNS server for VPN connections (e.g., your
            local network DNS or a preferred DNS to use while connected to the VPN).
          </p>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="vpn_dns_server_ip">
            {(field) => <field.FormInput label="DNS Server IP" />}
          </form.AppField>
          <ModalControls
            submitProps={{
              text: 'Continue',
              onClick: form.handleSubmit,
              loading: isPending,
            }}
          >
            <Button
              variant="outlined"
              text="Back"
              onClick={() => {
                useAutoAdoptionSetupWizardStore.setState(form.state.values);
                setActiveStep(AutoAdoptionSetupStep.UrlSettings);
              }}
            />
          </ModalControls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
