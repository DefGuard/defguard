import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { InteractiveBlock } from '../../../shared/defguard-ui/components/InteractiveBlock/InteractiveBlock';
import { Select } from '../../../shared/defguard-ui/components/Select/Select';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import type { PostureCheckVersionValues } from '../../PostureChecksPage/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

type Props = {
  versionValues: PostureCheckVersionValues;
};

export const AddPostureCheckClientVersionStep = ({ versionValues }: Props) => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const minimumClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumClientVersion,
  );
  const allowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.allowPrereleaseClient,
  );
  const setMinimumClientVersion = useAddPostureCheckWizardStore(
    (s) => s.setMinimumClientVersion,
  );
  const setAllowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.setAllowPrereleaseClient,
  );

  const versionOptions = versionValues.defguard.map((version) => ({
    key: version,
    label: m.posture_checks_wizard_client_version_option({ version }),
    value: version,
  }));

  const selectedVersion =
    versionOptions.find((option) => option.value === minimumClientVersion) ??
    versionOptions[versionOptions.length - 1];

  return (
    <WizardCard className="add-posture-check-client-version-step">
      <div className="client-version-track">
        <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgMuted}>
          {m.posture_checks_wizard_client_version_note()}
        </AppText>
        <Select
          options={versionOptions}
          value={selectedVersion}
          onChange={(option) => {
            setMinimumClientVersion(option.value);
          }}
        />
        <InteractiveBlock
          variant="checkbox"
          value={allowPrereleaseClient}
          title={m.posture_checks_wizard_client_version_prerelease_title()}
          content={m.posture_checks_wizard_client_version_prerelease_description()}
          onClick={() => {
            setAllowPrereleaseClient(!allowPrereleaseClient);
          }}
        />
      </div>
      <Controls>
        <Button text={m.controls_back()} variant="outlined" onClick={back} />
        <div className="right">
          <Button text={m.controls_continue()} onClick={next} />
        </div>
      </Controls>
    </WizardCard>
  );
};
