import { useNavigate } from '@tanstack/react-router';
import { Fragment } from 'react';
import { m } from '../../../paraglide/messages';
import api from '../../../shared/api/api';
import { Controls } from '../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../shared/components/wizard/WizardCard/WizardCard';
import { AppText } from '../../../shared/defguard-ui/components/AppText/AppText';
import { Button } from '../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../shared/defguard-ui/components/Divider/Divider';
import { Icon } from '../../../shared/defguard-ui/components/Icon';
import { InfoBanner } from '../../../shared/defguard-ui/components/InfoBanner/InfoBanner';
import { TextStyle, ThemeVariable } from '../../../shared/defguard-ui/types';
import { openModal } from '../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../shared/hooks/modalControls/modalTypes';
import { closeAddPostureCheckWizard } from '../navigation';
import { buildAddPostureCheckRequest } from '../payload';
import {
  buildClientSummarySection,
  buildOperatingSystemSummarySection,
} from '../summary';
import { useAddPostureCheckWizardStore } from '../useAddPostureCheckWizardStore';

export const AddPostureCheckSummaryStep = () => {
  const back = useAddPostureCheckWizardStore((s) => s.back);
  const description = useAddPostureCheckWizardStore((s) => s.description);
  const configuredOperatingSystems = useAddPostureCheckWizardStore(
    (s) => s.configuredOperatingSystems,
  );
  const minimumClientVersion = useAddPostureCheckWizardStore(
    (s) => s.minimumClientVersion,
  );
  const allowPrereleaseClient = useAddPostureCheckWizardStore(
    (s) => s.allowPrereleaseClient,
  );
  const name = useAddPostureCheckWizardStore((s) => s.name);
  const operatingSystemState = useAddPostureCheckWizardStore(
    (s) => s.operatingSystemState,
  );
  const navigate = useNavigate();

  const requestData = buildAddPostureCheckRequest({
    allowPrereleaseClient,
    configuredOperatingSystems,
    description,
    minimumClientVersion,
    name,
    operatingSystemState,
  });

  const sections = [
    ...configuredOperatingSystems.map((operatingSystem) =>
      buildOperatingSystemSummarySection(
        operatingSystem,
        operatingSystemState[operatingSystem],
      ),
    ),
    buildClientSummarySection(minimumClientVersion, allowPrereleaseClient),
  ];

  return (
    <WizardCard className="add-posture-check-summary-step">
      <div className="summary-track">
        <InfoBanner
          icon="info-outlined"
          text={m.posture_checks_wizard_summary_banner()}
        />
        <div className="summary-sections">
          {sections.map((section, index) => (
            <Fragment key={section.label}>
              <div className="summary-row">
                <div className="summary-heading">
                  <Icon icon={section.icon} size={16} />
                  <AppText font={TextStyle.TBodySm400} color={ThemeVariable.FgNeutral}>
                    {section.label}
                  </AppText>
                </div>
                <div className="summary-lines">
                  {section.lines.map((line) => (
                    <div className="summary-line" key={line.text}>
                      <Icon icon="status-check" size={16} />
                      <AppText
                        font={
                          line.emphasized ? TextStyle.TBodySm500 : TextStyle.TBodySm400
                        }
                        color={
                          line.emphasized
                            ? ThemeVariable.FgDefault
                            : ThemeVariable.FgFaded
                        }
                      >
                        {line.text}
                      </AppText>
                    </div>
                  ))}
                </div>
              </div>
              {index < sections.length - 1 && <Divider />}
            </Fragment>
          ))}
        </div>
      </div>
      <Controls>
        <Button text={m.controls_back()} variant="outlined" onClick={back} />
        <div className="right">
          <Button
            text={m.posture_checks_wizard_summary_submit()}
            onClick={() => {
              openModal(ModalName.ConfirmAction, {
                title: m.posture_checks_wizard_confirm_modal_title(),
                contentMd: m.posture_checks_wizard_confirm_modal_content(),
                actionPromise: () => api.devicePosture.addDevicePosture(requestData),
                invalidateKeys: [['device-posture']],
                submitProps: {
                  text: m.posture_checks_wizard_summary_submit(),
                },
                onSuccess: () => {
                  closeAddPostureCheckWizard(navigate);
                },
              });
            }}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
