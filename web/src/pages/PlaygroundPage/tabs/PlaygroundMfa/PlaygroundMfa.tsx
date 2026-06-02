import { useState } from 'react';
import { Card } from '../../../../shared/components/Card/Card';
import { LocationMfaConfiguration } from '../../../../shared/components/LocationMfaConfiguration/LocationMfaConfiguration';
import type { LocationMfaConfigurationStepData } from '../../../../shared/components/LocationMfaConfiguration/types';

export const PlaygroundMfa = () => {
  const [steps, setSteps] = useState<LocationMfaConfigurationStepData[]>([
    { id: 'step-1', order: 1, factors: ['email', 'totp'] },
    { id: 'step-2', order: 2, factors: ['biometry'] },
  ]);

  return (
    <div id="tab-mfa" className="tab">
      <Card>
        <LocationMfaConfiguration steps={steps} onChange={setSteps} />
      </Card>
    </div>
  );
};
