import './style.scss';
import { useMutation, useQuery, useSuspenseQuery } from '@tanstack/react-query';
import { useNavigate } from '@tanstack/react-router';
import { cloneDeep, omit } from 'lodash-es';
import { type Dispatch, Fragment, type SetStateAction, useState } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import type { ApiDevicePosture } from '../../../../shared/api/types';
import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { Card } from '../../../../shared/components/Card/Card';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { externalLink } from '../../../../shared/constants';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { Fold } from '../../../../shared/defguard-ui/components/Fold/Fold';
import { Icon, IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { Radio } from '../../../../shared/defguard-ui/components/Radio/Radio';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { TooltipContent } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../shared/defguard-ui/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../shared/defguard-ui/providers/tooltip/TooltipTrigger';
import { ThemeSpacing, ThemeVariable } from '../../../../shared/defguard-ui/types';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import {
  getDevicePostureQueryOptions,
  getLicenseInfoQueryOptions,
} from '../../../../shared/query';
import { canUseEnterpriseFeature } from '../../../../shared/utils/license';
import { buildOsSections } from '../../../../shared/utils/postureInfo';
import { useGatewayWizardStore } from '../../../GatewaySetupPage/useGatewayWizardStore';
import businessFeatureCardImage from '../../assets/business-feature-icon.png';
import actionCardImage from '../../assets/gateway-setup-action-card.png';
import { AddLocationPageStep } from '../../types';
import { useAddLocationStore } from '../../useAddLocationStore';

export const AddLocationPostureCheckStep = () => {
  const [addPostures, setAddPostures] = useState(false);
  const [showGateway, setShowGateway] = useState(false);
  const [selected, setSelected] = useState<Set<number>>(new Set());
  const { data: licenseInfo } = useQuery(getLicenseInfoQueryOptions);
  const { data: postures } = useSuspenseQuery({
    queryFn: api.devicePosture.getDevicePostures,
    queryKey: ['device-posture'],
  });
  const canUseEnterprise =
    licenseInfo === undefined ? undefined : canUseEnterpriseFeature(licenseInfo).result;
  const postureLocked = isPresent(canUseEnterprise) && !canUseEnterprise;
  const hasPostures = postures.length > 0;
  const canAssignPostures = canUseEnterprise === true && hasPostures;
  const navigate = useNavigate();

  const { mutate, isPending } = useMutation({
    mutationFn: api.location.addLocation,
    meta: {
      invalidate: [['network'], ['enterprise_info']],
    },
    onSuccess: ({ data }) => {
      if (showGateway) {
        useGatewayWizardStore.getState().start({ network_id: data.id });
        navigate({ to: '/setup-gateway', replace: true }).then(() => {
          setTimeout(() => {
            useAddLocationStore.getState().reset();
          }, 100);
        });
      } else {
        navigate({ to: '/locations', replace: true }).then(() => {
          setTimeout(() => {
            useAddLocationStore.getState().reset();
          }, 100);
        });
      }
    },
  });

  const handleSubmit = () => {
    const storageState = cloneDeep(
      omit(useAddLocationStore.getState(), [
        'start',
        'reset',
        'activeStep',
        'locationType',
      ]),
    );
    storageState.posture_checks = Array.from(selected);
    mutate(storageState);
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
      <Divider spacing={ThemeSpacing.Xl2} />
      <ActionCard
        imageSrc={actionCardImage}
        title={m.add_location_firewall_gateway_activation_title()}
        subtitle={m.add_location_firewall_gateway_activation_subtitle()}
      >
        <Checkbox
          text={m.add_location_firewall_gateway_activation_checkbox()}
          active={showGateway}
          onClick={() => {
            setShowGateway((s) => !s);
          }}
        />
      </ActionCard>
      <Controls>
        <Button
          variant="outlined"
          text={m.controls_back()}
          onClick={() => {
            useAddLocationStore.setState({
              activeStep: AddLocationPageStep.Firewall,
            });
          }}
        />
        <div className="right">
          <Button
            testId="create-location"
            text={m.add_location_create_location()}
            loading={isPending}
            onClick={() => {
              handleSubmit();
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
          <Checkbox
            active={selected.has(posture.id)}
            text={posture.name}
            onClick={() => {
              const next = new Set(selected);
              if (next.has(posture.id)) {
                next.delete(posture.id);
              } else {
                next.add(posture.id);
              }

              onChange(next);
            }}
          />
          <TooltipProvider>
            <TooltipTrigger>
              <div
                className="helper"
                style={{
                  width: 20,
                  height: 20,
                }}
              >
                <Icon
                  icon={IconKind.Help}
                  size={20}
                  staticColor={ThemeVariable.FgMuted}
                />
              </div>
            </TooltipTrigger>
            <TooltipContent variant="light" className="posture-check-helper-tooltip">
              <PostureCheckHelper postureCheckId={posture.id} />
            </TooltipContent>
          </TooltipProvider>
          <Divider spacing={ThemeSpacing.Md} />
        </Fragment>
      ))}
    </Card>
  );
};

interface PostureCheckHelperProps {
  postureCheckId: number;
}

const PostureCheckHelper = ({ postureCheckId }: PostureCheckHelperProps) => {
  const { data: postureCheck } = useSuspenseQuery(
    getDevicePostureQueryOptions(postureCheckId),
  );
  const osSections = buildOsSections(postureCheck);

  return (
    <div className="posture-check-helper">
      <div className="posture-check-helper-sections">
        {osSections.map((section, index) => (
          <Fragment key={section.name}>
            {index > 0 && <Divider />}
            <div className="posture-check-helper-section">
              <p className="posture-check-helper-label">{section.name}</p>
              <div className="posture-check-helper-content">
                {section.rows
                  .flatMap((detail) =>
                    Array.isArray(detail.value) ? detail.value : [detail.value],
                  )
                  .map((line, lineIndex) => (
                    <p
                      className="posture-check-helper-line"
                      key={`${section.name}-${lineIndex}`}
                    >
                      {line}
                    </p>
                  ))}
              </div>
            </div>
          </Fragment>
        ))}
      </div>
    </div>
  );
};
