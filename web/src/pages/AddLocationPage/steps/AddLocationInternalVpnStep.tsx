import { useQuery } from '@tanstack/react-query';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../shared/defguard-ui/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { networkSize } from '../../../shared/utils/network';
import { Validate } from '../../../shared/validate';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z.object({
  address: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required())
    .superRefine((val, ctx) => {
      if (!Validate.any(val, [Validate.CIDRv4, Validate.CIDRv6], true)) {
        ctx.addIssue({ code: 'custom', message: m.form_error_invalid() });
        return;
      }
      const addresses = val.split(',').map((a) => a.trim());
      if (addresses.some((a) => Validate.isNetworkAddress(a))) {
        ctx.addIssue({ code: 'custom', message: m.form_error_network_address() });
      } else if (addresses.some((a) => Validate.isBroadcastAddress(a))) {
        ctx.addIssue({ code: 'custom', message: m.form_error_broadcast_address() });
      }
    }),
  allowed_ips: z
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
  dns: z
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

export const AddLocationInternalVpnStep = () => {
  const { data: devices } = useQuery({
    queryKey: ['device', 'all'],
    queryFn: api.device.getDevices,
    select: (resp) => resp.data,
  });

  const defaultValues = useAddLocationStore(
    useShallow(
      (s): FormFields => ({
        allowed_ips: s.allowed_ips,
        dns: s.dns,
        address: s.address,
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
      const deviceCount = Array.isArray(devices) ? devices.length : 0;
      const network_size = networkSize(value.address);
      if (deviceCount > network_size) {
        Snackbar.error(
          m.location_error_network_too_small({ network_size, device_count: deviceCount }),
        );
        return;
      }
      useAddLocationStore.setState({
        ...value,
        allowed_ips: value.allowed_ips ?? '',
        activeStep: AddLocationPageStep.NetworkSettings,
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
          <DescriptionBlock title={m.add_location_internal_vpn_gateway_address_title()}>
            <p>{m.add_location_internal_vpn_gateway_address_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="address">
            {(field) => (
              <field.FormInput
                required
                label={m.add_location_internal_vpn_label_address()}
                helper={m.add_location_internal_vpn_helper_address()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title={m.add_location_internal_vpn_allowed_ips_title()}>
            <p>{m.add_location_internal_vpn_allowed_ips_description()}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="allowed_ips">
            {(field) => (
              <field.FormInput
                label={m.add_location_internal_vpn_label_allowed_ips()}
                helper={m.add_location_internal_vpn_helper_allowed_ips()}
              />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="dns">
            {(field) => (
              <field.FormInput
                label={m.add_location_internal_vpn_label_dns()}
                helper={m.add_location_internal_vpn_helper_dns()}
              />
            )}
          </form.AppField>
          <Controls>
            <Button
              variant="outlined"
              text={m.controls_back()}
              onClick={() => {
                useAddLocationStore.setState({
                  activeStep: AddLocationPageStep.Start,
                  ...form.state.values,
                  allowed_ips: form.state.values.allowed_ips ?? '',
                });
              }}
            />
            <div className="right">
              <Button
                text={m.controls_continue()}
                testId="continue"
                onClick={() => {
                  form.handleSubmit();
                }}
              />
            </div>
          </Controls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
