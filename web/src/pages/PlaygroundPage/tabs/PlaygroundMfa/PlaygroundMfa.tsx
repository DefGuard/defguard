import { useState } from 'react';
import { Card } from '../../../../shared/components/Card/Card';
import { MfaConfiguration } from '../../../../shared/components/MfaConfiguration/MfaConfiguration';
import type { MfaConfigurationStepData } from '../../../../shared/components/MfaConfiguration/types';

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
    </div>
  );
};
