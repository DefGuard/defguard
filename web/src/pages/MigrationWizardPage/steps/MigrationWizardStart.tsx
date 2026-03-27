import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { IconKind } from '../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { RenderMarkdown } from '../../../shared/defguard-ui/components/RenderMarkdown/RenderMarkdown';
import { SizedBox } from '../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../shared/defguard-ui/types';
import { useMigrationWizardStore } from '../store/useMigrationWizardStore';

export const MigrationWizardStart = () => {
  return (
    <>
      <SizedBox height={ThemeSpacing.Lg} />
      <InfoBanner
        icon={IconKind.InfoOutlined}
        variant="warning"
        text={m.migration_wizard_start_warning()}
      />
      <SizedBox height={ThemeSpacing.Lg} />
      <RenderMarkdown
        containerProps={{
          id: 'migration-start-md-block',
        }}
        content={`${m.migration_wizard_start_explain_1()}</br></br>${m.migration_wizard_start_explain_2()}`}
      />
      <SizedBox height={ThemeSpacing.Xl} />
      <Divider spacing={ThemeSpacing.Xs} />
      <SizedBox height={ThemeSpacing.Xl} />
      <Controls>
        <div className="left">
          <Button
            text={m.migration_wizard_start_button()}
            onClick={() => {
              useMigrationWizardStore.getState().next();
            }}
          />
        </div>
      </Controls>
    </>
  );
};
