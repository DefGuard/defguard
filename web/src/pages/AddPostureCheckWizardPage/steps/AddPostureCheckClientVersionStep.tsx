import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Checkbox } from '../../../shared/defguard-ui/components/Checkbox/Checkbox';
import { Select } from '../../../shared/defguard-ui/components/Select/Select';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import { postureCheckVersionValues } from '../../PostureChecksPage/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckClientVersionStep = () => {
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

  const versionOptions = postureCheckVersionValues.defguard.map((version) => ({
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
        <Checkbox
          active={allowPrereleaseClient}
          onClick={() => {
            setAllowPrereleaseClient(!allowPrereleaseClient);
          }}
        >
          <div className="client-version-checkbox-copy">
            <AppText font={TextStyle.TBodySm500}>
              {m.posture_checks_wizard_client_version_prerelease_title()}
            </AppText>
            <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
              {m.posture_checks_wizard_client_version_prerelease_description()}
            </AppText>
          </div>
        </Checkbox>
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
