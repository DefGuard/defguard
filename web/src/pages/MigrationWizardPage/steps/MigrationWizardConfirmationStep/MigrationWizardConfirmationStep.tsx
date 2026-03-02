import { useState } from 'react';
import { m } from '../../../../paraglide/messages';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { ActionableSection } from '../../../../shared/defguard-ui/components/ActionableSection/ActionableSection';
import { AppText } from '../../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { RenderMarkdown } from '../../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { Snackbar } from '../../../../shared/defguard-ui/providers/snackbar/snackbar';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../../shared/defguard-ui/types';
import prepareNetworkImage from './assets/prepare-network.png';

export const MigrationWizardConfirmationStep = () => {
  const [confirm, setConfirm] = useState(false);

  return (
    <WizardCard>
      <AppText font={TextStyle.TTitleH4} color={ThemeVariable.FgSuccess}>
        {`Initial system migration are complete.`}
      </AppText>
      <SizedBox height={ThemeSpacing.Sm} />
      <AppText font={TextStyle.TBodyPrimary400} color={ThemeVariable.FgNeutral}>
        {`You've completed the first stage of the migration. Defguard is almost ready to go.`}
      </AppText>
      <Divider spacing={ThemeSpacing.Xl2} />
      <AppText font={TextStyle.TBodyPrimary500} color={ThemeVariable.FgFaded}>
        {`You currently have X VPN locations configured. These locations must be upgraded.`}
      </AppText>
      <SizedBox height={ThemeSpacing.Md} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
        {`A key architectural change in Defguard 2.0 is that the Core now initiates connections to the gateways (in 1.x, gateways connected to the Core). As a result, in addition to upgrading the gateway components, you must update your firewall rules to:`}
      </AppText>
      <SizedBox height={ThemeSpacing.Lg} />
      <ul id="upgrade-guide-list">
        <li>{`Allow connections from the Core to the gateways on port 5055 tcp.`}</li>
        <li>{`Block connections from the gateways to the Core.`}</li>
      </ul>
      <Divider spacing={ThemeSpacing.Lg} />
      <RenderMarkdown
        containerProps={{
          id: 'confirm-improve-notice',
        }}
        content={`**This change significantly improves the overall security of Defguard deployments**. 
You can [read more about it in the documentation](https://docs.defguard.net)`}
      />
      <SizedBox height={ThemeSpacing.Xl2} />
      <ActionableSection
        imageSrc={prepareNetworkImage}
        title={`Prepare your network`}
        subtitle={`Please prepare all required network and firewall changes before starting the migration. Once ready, we’ll begin adopting the upgraded gateway components for each VPN location.`}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Checkbox
        active={confirm}
        onClick={() => {
          setConfirm((s) => !s);
        }}
        text={`I have changed all my gateways firewall rules and network setup`}
      />
      <Controls>
        <div className="right">
          <Button
            variant="primary"
            text={m.controls_finish()}
            disabled={!confirm}
            onClick={() => {
              Snackbar.default(`TODO`);
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
