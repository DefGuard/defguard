import { m } from '../../../paraglide/messages';
import { Controls } from '../../../shared/components/Controls/Controls';
import {
  PostureCheckDefguardSection,
  type PostureCheckEditorValues,
} from '../../../shared/components/postureChecksEditor/PostureCheckEditorSections';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import type { PostureCheckVersionValues } from '../../PostureChecksPage/types';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

type Props = {
  versionValues: PostureCheckVersionValues;
};

export const AddPostureCheckClientVersionStep = ({ versionValues }: Props) => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const next = useAddPostureCheckWizardStore((s) => s.next);
  const minimumDesktopClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumDesktopClientVersion,
  );
  const minimumMobileClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumMobileClientVersion,
  );
  const allowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.allowPrereleaseClient,
  );
  const setMinimumDesktopClientVersion = useAddPostureCheckWizardStore(
    (s) => s.setMinimumDesktopClientVersion,
  );
  const setMinimumMobileClientVersion = useAddPostureCheckWizardStore(
    (s) => s.setMinimumMobileClientVersion,
  );
  const setAllowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.setAllowPrereleaseClient,
  );
  const name = useAddPostureCheckWizardStore((s) => s.name);
  const description = useAddPostureCheckWizardStore((s) => s.description);
  const configuredOperatingSystems = useAddPostureCheckWizardStore(
    (s) => s.configuredOperatingSystems,
  );
  const operatingSystemState = useAddPostureCheckWizardStore(
    (s) => s.operatingSystemState,
  );

  const values: PostureCheckEditorValues = {
    allowPrereleaseClient,
    configuredOperatingSystems,
    description,
    locations: new Set<number>(),
    minimumDesktopClientVersion,
    minimumMobileClientVersion,
    name,
    operatingSystemState,
  };

  return (
    <WizardCard className="add-posture-check-client-version-step">
      <div className="client-version-track">
        <PostureCheckDefguardSection
          values={values}
          versionValues={versionValues}
          updateValues={(updater) => {
            const nextValues = updater(values);
            setMinimumDesktopClientVersion(nextValues.minimumDesktopClientVersion);
            setMinimumMobileClientVersion(nextValues.minimumMobileClientVersion);
            setAllowPrereleaseClient(nextValues.allowPrereleaseClient);
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
