import { Controls } from '../../../shared/components/Controls/Controls';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { IconKind } from '../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { RenderMarkdown } from '../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import {
  TextStyle,
  ThemeSpacing,
  ThemeVariable,
} from '../../../shared/defguard-ui/types';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardStart = () => {
  return (
    <>
      <SizedBox height={ThemeSpacing.Lg} />
      <InfoBanner
        icon={IconKind.InfoOutlined}
        variant="warning"
        text={`IMPORTANT: Until you finish this migration process your VPN Locations will not work!.`}
      />
      <SizedBox height={ThemeSpacing.Lg} />
      <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgFaded}>
        <RenderMarkdown content={`${explain1}</br></br>${explain2}`} />
      </AppText>
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <div className="left">
          <Button
            text="Start migration process"
            onClick={() => {
              useMigrationWizardStore.getState().next();
            }}
          />
        </div>
      </Controls>
    </>
  );
};

const explain1 = `We will first automatically upgrade the Core instance (what you see now), followed by the public communication component, Edge (called Proxy prior to 2.0).`;

const explain2 = `Next, each VPN location must be upgraded. This will likely require manual changes to your internal network (firewall rules), as the Core ↔ Gateway communication has changed: the Core now initiates the connection to the Gateway (in 1.x Gateway connected to Core).`;
