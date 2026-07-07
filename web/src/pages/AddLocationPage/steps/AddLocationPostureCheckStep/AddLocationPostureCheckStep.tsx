import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { type Dispatch, Fragment, type SetStateAction, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { type ApiDevicePosture, LicenseFeature } from '../../../../shared/api/types';
import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { Card } from '../../../../shared/components/Card/Card';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { renderPostureCheckSelectionItem } from '../../../../shared/components/PostureCheckSelectionItem/PostureCheckSelectionItem';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { externalLink } from '../../../../shared/constants';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Radio } from '../../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { getLicenseInfoQueryOptions } from '../../../../shared/query';
import { canUseEnterpriseFeature } from '../../../../shared/utils/license';
import businessFeatureCardImage from '../../assets/business-feature-icon.png';
import { AddLocationPageStep } from '../../types';
import { useAddLocationStore } from '../../useAddLocationStore';

export const AddLocationPostureCheckStep = () => {
  const storedPostureChecks = useAddLocationStore.getState().posture_checks;
  const [addPostures, setAddPostures] = useState(storedPostureChecks.length > 0);
  const [selected, setSelected] = useState<Set<number>>(new Set(storedPostureChecks));
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const canUseEnterprise =
    licenseInfo === undefined
      ? undefined
      : canUseEnterpriseFeature(licenseInfo, LicenseFeature.DevicePosture).result;
  const postureLocked = isPresent(canUseEnterprise) && !canUseEnterprise;
  const { data: postures } = useQuery({
    queryFn: api.devicePosture.getDevicePostures,
    queryKey: ['device-posture'],
    enabled: canUseEnterprise === true,
  });
  const hasPostures = isPresent(postures) && postures.length > 0;
  const canAssignPostures = canUseEnterprise === true && hasPostures;

  const handleContinue = () => {
    useAddLocationStore.setState({
      posture_checks: addPostures ? Array.from(selected) : [],
      activeStep: AddLocationPageStep.Firewall,
    });
  };

  return (
    <WizardCard>
      {postureLocked && (
        <>
          <ActionCard
            imageSrc={businessFeatureCardImage}
            title=""
            subtitle={m.add_location_postures_enterprise_only()}
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
      {!hasPostures && canUseEnterprise && (
        <>
          <InfoBanner
            icon="warning-outlined"
            variant="warning"
            text={m.add_location_postures_create_postures_warning()}
          />
          <SizedBox height={ThemeSpacing.Xl2} />
        </>
      )}
      <Radio
        active={!addPostures}
        onClick={() => {
          setAddPostures(false);
        }}
        text={m.add_location_postures_dont_assign()}
        disabled={!canAssignPostures}
      />
      <SizedBox height={ThemeSpacing.Md} />
      <Radio
        active={addPostures}
        onClick={() => {
          setAddPostures(true);
        }}
        text={m.add_location_postures_assign()}
        disabled={!canAssignPostures}
      />
      <Fold open={addPostures}>
        <SizedBox height={ThemeSpacing.Xl2} />
        {postures && (
          <PostureSelection
            postures={postures}
            selected={selected}
            onChange={setSelected}
          />
        )}
      </Fold>
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.AccessControl,
            });
          }}
        />
        <div className="right">
          <Button
            testId="posture-continue"
            text={m.controls_continue()}
            onClick={() => {
              handleContinue();
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};

interface PostureSelectionProps {
  postures: ApiDevicePosture[];
  selected: Set<number>;
  onChange: Dispatch<SetStateAction<Set<number>>>;
}
const PostureSelection = ({ postures, selected, onChange }: PostureSelectionProps) => {
  return (
    <Card className="posture-selection">
      {postures.map((posture) => (
        <Fragment key={posture.id}>
          <div className="posture-selection-row">
            {renderPostureCheckSelectionItem({
              active: selected.has(posture.id),
              option: {
                id: posture.id,
                label: posture.name,
                meta: posture,
              },
              onClick: () => {
                const next = new Set(selected);
                if (next.has(posture.id)) {
                  next.delete(posture.id);
                } else {
                  next.add(posture.id);
                }

                onChange(next);
              },
            })}
          </div>
          <Divider spacing={ThemeSpacing.Md} />
        </Fragment>
      ))}
    </Card>
  );
};
