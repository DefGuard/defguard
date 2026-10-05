import { useState } from 'react';
import type { MfaFlowStepMethods } from '../../../../shared/api/types';
import { MfaMethodAvailabilityReason } from '../../../../shared/api/types';
import { Card } from '../../../../shared/components/Card/Card';
import { MfaConfiguration } from '../../../../shared/components/MfaConfiguration/MfaConfiguration';
import type { MfaConfigurationStepData } from '../../../../shared/components/MfaConfiguration/types';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { ThemeVariable } from '../../../../shared/defguard-ui/types';
import { mfaFlowUnavailableText } from '../../../../shared/utils/mfaFlowSteps';
import { MfaFlowAssignmentCard } from '../../../EditLocationPage/components/LocationMfaSection/components/MfaFlowAssignmentCard/MfaFlowAssignmentCard';

const flowSteps: MfaFlowStepMethods[] = [
  { methods: ['email', 'totp'] },
  { methods: ['mobileapprove'] },
];

const manyGroups = [
  'admin',
  'developers',
  'support',
  'sales',
  'marketing',
  'finance',
  'contractors',
  'interns',
];

const leading = (
  <Icon icon={IconKind.Groups} size={20} staticColor={ThemeVariable.FgAction} />
);

const noop = () => {};

export const PlaygroundMfa = () => {
  const [steps, setSteps] = useState<MfaConfigurationStepData[]>([
    { id: 'step-1', methods: ['email', 'totp'] },
    { id: 'step-2', methods: ['mobileapprove'] },
  ]);

  return (
    <div id="tab-mfa" className="tab">
      <Card>
        <MfaConfiguration steps={steps} onChange={setSteps} />
      </Card>
      <Card>
        <div style={{ display: 'flex', flexFlow: 'column', rowGap: 24, maxWidth: 640 }}>
          <MfaFlowAssignmentCard
            title="Default flow"
            steps={flowSteps}
            chips={['All groups']}
            leading={leading}
            editLabel="Edit"
            onEdit={noop}
          />
          <MfaFlowAssignmentCard
            title="Override with remove"
            steps={flowSteps}
            chips={['admin', 'developers']}
            leading={leading}
            editLabel="Edit"
            onEdit={noop}
            removeLabel="Remove"
            onRemove={noop}
          />
          <MfaFlowAssignmentCard
            title="Many groups (foldable)"
            steps={flowSteps}
            chips={manyGroups}
            leading={leading}
            editLabel="Edit"
            onEdit={noop}
            removeLabel="Remove"
            onRemove={noop}
          />
          {[
            MfaMethodAvailabilityReason.Licensed,
            MfaMethodAvailabilityReason.SmtpNotConfigured,
            MfaMethodAvailabilityReason.OidcProviderMissing,
          ].map((reason) => (
            <MfaFlowAssignmentCard
              key={reason}
              title={`Unavailable: ${reason}`}
              steps={flowSteps}
              chips={manyGroups}
              unavailableText={mfaFlowUnavailableText(reason)}
              leading={leading}
              editLabel="Edit"
              onEdit={noop}
              removeLabel="Remove"
              onRemove={noop}
            />
          ))}
        </div>
      </Card>
    </div>
  );
};
