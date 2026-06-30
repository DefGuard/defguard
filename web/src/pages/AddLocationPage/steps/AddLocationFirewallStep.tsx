import { useQuery } from '@tanstack/react-query';
import { useCallback, useMemo, useState } from 'react';
import { m } from '../../../paraglide/messages';
import { ActionCard } from '../../../shared/components/ActionCard/ActionCard';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { externalLink } from '../../../shared/constants';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Radio } from '../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { isPresent } from '../../../shared/defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../../shared/query';
import { canUseBusinessFeature } from '../../../shared/utils/license';
import businessFeatureCardImage from '../assets/business-feature-icon.png';
import { AddLocationPageStep } from '../types';
import { useAddLocationStore } from '../useAddLocationStore';

type Choice = 'disable' | 'enabled-allowed' | 'enabled-denied';

export const AddLocationFirewallStep = () => {
  const [state, setState] = useState<Choice>('disable');

  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const canUseFeature = useMemo(() => {
    if (licenseInfo === undefined) return undefined;
    return canUseBusinessFeature(licenseInfo).result;
  }, [licenseInfo]);
  const firewallLocked = isPresent(canUseFeature) && !canUseFeature;

  const saveChanges = useCallback((value: Choice) => {
    const enabled = value !== 'disable';
    const allowed = value === 'enabled-allowed';
    useAddLocationStore.setState({
      acl_enabled: enabled,
      acl_default_allow: allowed,
    });
  }, []);

  const handleSubmit = () => {
    const enabled = state !== 'disable';
    const allowed = state === 'enabled-allowed';
    useAddLocationStore.setState({
      acl_enabled: enabled,
      acl_default_allow: allowed,
      activeStep: AddLocationPageStep.PostureCheck,
    });
  };

  return (
    <WizardCard>
      {firewallLocked && (
        <>
          <ActionCard
            imageSrc={businessFeatureCardImage}
            title=""
            subtitle={m.add_location_firewall_no_license_subtitle()}
          >
            <a href={externalLink.defguard.pricing} target="_blank" rel="noreferrer">
              <Button
                variant="outlined"
                text={m.license_see_other_plans()}
                iconRight="open-in-new-window"
              />
            </a>
          </ActionCard>
          <Divider spacing={ThemeSpacing.Xl2} />
        </>
      )}
      <Radio
        active={state === 'disable'}
        onClick={() => {
          setState('disable');
        }}
        text={m.location_firewall_option_disabled()}
        disabled={firewallLocked}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={state === 'enabled-allowed'}
        onClick={() => {
          setState('enabled-allowed');
        }}
        text={m.location_firewall_option_default_allow()}
        disabled={firewallLocked}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={state === 'enabled-denied'}
        onClick={() => {
          setState('enabled-denied');
        }}
        text={m.location_firewall_option_default_deny()}
        disabled={firewallLocked}
      />
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            saveChanges(state);
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.AccessControl,
            });
          }}
        />
        <div className="right">
          <Button
            testId="firewall-continue"
            text={m.controls_continue()}
            onClick={() => {
              handleSubmit();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
