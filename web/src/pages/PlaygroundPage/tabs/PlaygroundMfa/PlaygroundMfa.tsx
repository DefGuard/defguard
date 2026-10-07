import { useState } from 'react';
import type {
  MfaFlowAssignment,
  MfaFlowListItemResponse,
  MfaFlowStepMethods,
} from '../../../../shared/api/types';
import { MfaMethodAvailabilityReason } from '../../../../shared/api/types';
import { Card } from '../../../../shared/components/Card/Card';
import { MfaConfiguration } from '../../../../shared/components/MfaConfiguration/MfaConfiguration';
import type { MfaConfigurationStepData } from '../../../../shared/components/MfaConfiguration/types';
import { IconKind } from '../../../../shared/defguard-ui/components/Icon';
import { Icon } from '../../../../shared/defguard-ui/components/Icon/Icon';
import { ThemeVariable } from '../../../../shared/defguard-ui/types';
import { mfaFlowUnavailableText } from '../../../../shared/utils/mfaFlowSteps';
import { MfaFlowAssignmentCard } from '../../../EditLocationPage/components/LocationMfaSection/components/MfaFlowAssignmentCard/MfaFlowAssignmentCard';
import { LocationMfaSection } from '../../../EditLocationPage/components/LocationMfaSection/LocationMfaSection';

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
  'devops',
  'security',
  'qa',
  'design',
  'legal',
  'hr',
  'operations',
  'customer-success',
  'partners',
  'executives',
  'infrastructure',
  'data-science',
  'mobile',
  'frontend',
  'backend',
  'research',
];

const groupOptions = manyGroups.map((label, index) => ({ id: index + 1, label }));

const mockFlow = (
  id: number,
  title: string,
  unavailableReason: MfaFlowListItemResponse['unavailable_reason'] = null,
): MfaFlowListItemResponse => ({
  id,
  title,
  step_count: flowSteps.length,
  steps: flowSteps.map((step, index) => ({ ...step, id: index + 1, position: index })),
  created_at: '',
  updated_at: '',
  unavailable_reason: unavailableReason,
});

const locationFlows: MfaFlowListItemResponse[] = [
  mockFlow(1, 'Default flow'),
  mockFlow(2, 'Engineering flow'),
  mockFlow(3, 'Support flow'),
  mockFlow(4, 'Email only flow', MfaMethodAvailabilityReason.SmtpNotConfigured),
  mockFlow(5, 'Unassigned flow'),
];

const initialLocationAssignments: MfaFlowAssignment[] = [
  { flow_id: 2, is_default: false, group_ids: groupOptions.map((option) => option.id) },
  { flow_id: 3, is_default: false, group_ids: [3, 4] },
  { flow_id: 4, is_default: false, group_ids: [6, 7, 8] },
  { flow_id: 1, is_default: true, group_ids: [] },
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
  const [locationAssignments, setLocationAssignments] = useState(
    initialLocationAssignments,
  );

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
      <Card>
        <div style={{ maxWidth: 640 }}>
          <LocationMfaSection
            assignments={locationAssignments}
            flows={locationFlows}
            groupOptions={groupOptions}
            canUseEnterprise
            onChange={setLocationAssignments}
          />
        </div>
      </Card>
    </div>
  );
};
