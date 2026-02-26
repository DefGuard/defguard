import { useQuery } from '@tanstack/react-query';
import { useMemo } from 'react';
import z from 'zod';
import { useShallow } from 'zustand/react/shallow';
import { m } from '../../../paraglide/messages';
import { DescriptionBlock } from '../../../shared/components/DescriptionBlock/DescriptionBlock';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { ButtonsGroup } from '../../../shared/defguard-ui/components/ButtonsGroup/ButtonsGroup';
import { ModalControls } from '../../../shared/defguard-ui/components/ModalControls/ModalControls';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Toggle } from '../../../shared/defguard-ui/components/Toggle/Toggle';
import { TooltipContent } from '../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { useAppForm } from '../../../shared/form';
import { formChangeLogic } from '../../../shared/formLogic';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature } from '../../../shared/utils/license';
import { Validate } from '../../../shared/validate';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

const formSchema = z.object({
  address: z
    .string(m.form_error_required())
    .trim()
    .min(1, m.form_error_required())
    .refine(
      (value) => Validate.any(value, [Validate.CIDRv4, Validate.CIDRv6], true),
      m.form_error_invalid(),
    ),
  allowed_ips: z.string(m.form_error_required()).trim(),
  dns: z.string().nullable(),
});

type FormFields = z.infer<typeof formSchema>;

export const AddLocationInternalVpnStep = () => {
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const canUseBusiness = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);
  const firewallRulesToggleLocked = isPresent(canUseBusiness) && !canUseBusiness;

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
      useAddLocationStore.setState({
        ...value,
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
          <DescriptionBlock title="Gateway address">
            <p>
              {
                'The VPN network will be derived from this address (e.g., 10.10.10.1 → 10.10.10.0). You can specify multiple addresses separated by commas. The first one is used as the primary address for device IP assignment.'
              }
            </p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="address">
            {(field) => (
              <field.FormInput required label="Gateway VPN IP address and netmask" />
            )}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Xl} />
          <DescriptionBlock title={`Allowed IP's`}>
            <p>{`List of addresses/masks that should be routed through the VPN network.`}</p>
          </DescriptionBlock>
          <SizedBox height={ThemeSpacing.Lg} />
          <form.AppField name="allowed_ips">
            {(field) => <field.FormInput required label={'Allowed IPs'} />}
          </form.AppField>
          <SizedBox height={ThemeSpacing.Lg} />
          <ButtonsGroup>
            <TooltipProvider disabled={!firewallRulesToggleLocked} placement="top">
              <TooltipTrigger>
                <div>
                  <Toggle // Does nothing now #TODO: implement generating allowed ips based on firewall rules
                    active={false}
                    disabled={firewallRulesToggleLocked}
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent>
                <p>{m.license_upgrade_business_tooltip()}</p>
              </TooltipContent>
            </TooltipProvider>
            <AppText
              as="span"
              font={TextStyle.TBodySm400}
              color={
                firewallRulesToggleLocked
                  ? ThemeVariable.FgDisabled
                  : ThemeVariable.FgDefault
              }
            >
              {m.add_location_internal_vpn_allowed_ips_from_firewall_rules()}
            </AppText>
          </ButtonsGroup>
          <SizedBox height={ThemeSpacing.Xl} />
          <form.AppField name="dns">
            {(field) => <field.FormInput label={'DNS'} />}
          </form.AppField>
          <ModalControls
            submitProps={{
              text: m.controls_continue(),
              testId: 'continue',
              onClick: () => {
                form.handleSubmit();
              },
            }}
          >
            <Button
              variant="outlined"
              text={m.controls_back()}
              onClick={() => {
                useAddLocationStore.setState({
                  activeStep: AddLocationPageStep.Start,
                  ...form.state.values,
                });
              }}
            />
          </ModalControls>
        </form.AppForm>
      </form>
    </WizardCard>
  );
};
