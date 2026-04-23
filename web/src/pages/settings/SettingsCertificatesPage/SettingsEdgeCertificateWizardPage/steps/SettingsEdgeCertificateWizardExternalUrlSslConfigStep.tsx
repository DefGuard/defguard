import { useQuery } from '@tanstack/react-query';
import { useCallback, useEffect, useState } from 'react';
import { m } from '../../../../../paraglide/messages';
import api from '../../../../../shared/api/api';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { LoadingStep } from '../../../../../shared/components/LoadingStep/LoadingStep';
import { WizardCard } from '../../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../../shared/defguard-ui/components/Button/Button';
import { CodeCard } from '../../../../../shared/defguard-ui/components/CodeCard/CodeCard';
import { Divider } from '../../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../../shared/defguard-ui/types';
import { useSSEController } from '../../../../../shared/hooks/useSSEController';
import { downloadFile } from '../../../../../shared/utils/download';
import caIcon from '../../../../SetupPage/assets/ca.png';
import '../../../../SetupPage/autoAdoption/steps/style.scss';
import { SettingsEdgeCertificateWizardStep } from '../types';
import { useSettingsEdgeCertificateWizardStore } from '../useSettingsEdgeCertificateWizardStore';

type AcmeStepId =
  | 'Connecting'
  | 'CheckingDomain'
  | 'ValidatingDomain'
  | 'IssuingCertificate'
  | 'Installing'
  | 'Done';

type AcmeEvent = {
  step: AcmeStepId;
  error: boolean;
  message?: string;
  logs?: string[];
};

type AcmeStepState = {
  currentStep: AcmeStepId | null;
  isComplete: boolean;
  isProcessing: boolean;
  isFailed: boolean;
  errorMessage: string | null;
  proxyLogs: string[];
};

const ACME_STEP_IDS: AcmeStepId[] = [
  'CheckingDomain',
  'Connecting',
  'ValidatingDomain',
  'IssuingCertificate',
  'Installing',
];

const defaultAcmeState: AcmeStepState = {
  currentStep: null,
  isComplete: false,
  isProcessing: false,
  isFailed: false,
  errorMessage: null,
  proxyLogs: [],
};

export const SettingsEdgeCertificateWizardExternalUrlSslConfigStep = () => {
  const sslType = useSettingsEdgeCertificateWizardStore((s) => s.external_ssl_type);
  const certInfo = useSettingsEdgeCertificateWizardStore((s) => s.external_ssl_cert_info);

  const [acmeState, setAcmeState] = useState<AcmeStepState>(defaultAcmeState);

  const { data: sslInfoData } = useQuery({
    queryKey: ['core', 'cert', 'ca'],
    queryFn: api.core.getCA,
    enabled: sslType === 'defguard_ca',
    select: (response) => response.data,
  });

  const handleAcmeEvent = useCallback((event: AcmeEvent) => {
    setAcmeState({
      currentStep: event.step,
      isComplete: event.step === 'Done',
      isProcessing: event.step !== 'Done' && !event.error,
      isFailed: Boolean(event.error),
      errorMessage: event.error
        ? (event.message ??
          m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_error_default())
        : null,
      proxyLogs: event.logs && event.logs.length > 0 ? [...event.logs] : [],
    });
  }, []);

  const sse = useSSEController<AcmeEvent>(
    '/api/v1/proxy/acme/stream',
    {},
    { onMessage: handleAcmeEvent },
  );

  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    if (sslType !== 'lets_encrypt') return;
    setAcmeState(defaultAcmeState);
    sse.start();
    return () => {
      sse.stop();
    };
  }, []);

  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    if (sslType === null) {
      useSettingsEdgeCertificateWizardStore.setState({
        activeStep: SettingsEdgeCertificateWizardStep.ExternalUrlSettings,
      });
    }
  }, []);

  const stepDone = useCallback(
    (stepId: AcmeStepId): boolean => {
      const stepIndex = ACME_STEP_IDS.indexOf(stepId);
      const currentIndex = acmeState.currentStep
        ? ACME_STEP_IDS.indexOf(acmeState.currentStep)
        : -1;
      return stepIndex < currentIndex || acmeState.isComplete;
    },
    [acmeState.isComplete, acmeState.currentStep],
  );

  const stepLoading = useCallback(
    (stepId: AcmeStepId): boolean => {
      return acmeState.isProcessing && acmeState.currentStep === stepId;
    },
    [acmeState.isProcessing, acmeState.currentStep],
  );

  const stepError = useCallback(
    (stepId: AcmeStepId): string | null => {
      if (acmeState.errorMessage && acmeState.currentStep === stepId) {
        return acmeState.errorMessage;
      }
      return null;
    },
    [acmeState.errorMessage, acmeState.currentStep],
  );

  const handleDownloadCaCert = () => {
    if (!sslInfoData?.ca_cert_pem) return;
    downloadFile(
      new Blob([sslInfoData.ca_cert_pem], { type: 'application/x-pem-file' }),
      'defguard-ca',
      'pem',
    );
  };

  const renderContent = () => {
    if (sslType === 'none') {
      return (
        <div className="ssl-result-card">
          <div className="ssl-result-card-header">
            <h3 className="green">
              {m.initial_setup_auto_adoption_external_url_ssl_no_ssl_title()}
            </h3>
            <p>{m.initial_setup_auto_adoption_external_url_ssl_no_ssl_description()}</p>
          </div>
          <Divider />
          <ul className="ssl-port-list">
            <li>{m.initial_setup_auto_adoption_external_url_ssl_no_ssl_port()}</li>
          </ul>
        </div>
      );
    }

    if (sslType === 'lets_encrypt') {
      const steps: { id: AcmeStepId; title: string }[] = [
        {
          id: 'CheckingDomain',
          title:
            m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_checking_domain(),
        },
        {
          id: 'Connecting',
          title: m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_connecting(),
        },
        {
          id: 'ValidatingDomain',
          title: m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_validating(),
        },
        {
          id: 'IssuingCertificate',
          title: m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_issuing(),
        },
        {
          id: 'Installing',
          title: m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_installing(),
        },
      ];

      return (
        <div className="ssl-result-card">
          <div>
            {steps.map((step) => (
              <LoadingStep
                key={step.id}
                title={step.title}
                loading={stepLoading(step.id)}
                success={stepDone(step.id)}
                error={!!stepError(step.id)}
                errorMessage={stepError(step.id) ?? undefined}
              >
                {acmeState.proxyLogs.length > 0 ? (
                  <>
                    <CodeCard
                      title={m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_error_log_title()}
                      value={acmeState.proxyLogs.join('\n')}
                      copy
                      download
                    />
                    <SizedBox height={ThemeSpacing.Xl} />
                  </>
                ) : null}
                <Controls>
                  <div className="left">
                    <Button
                      variant="primary"
                      text={m.edge_setup_adoption_controls_retry()}
                      onClick={() => {
                        setAcmeState(defaultAcmeState);
                        sse.restart();
                      }}
                      disabled={acmeState.isProcessing}
                    />
                  </div>
                </Controls>
              </LoadingStep>
            ))}
          </div>
        </div>
      );
    }

    if (sslType === 'defguard_ca') {
      return (
        <div className="ssl-result-validated-card">
          <div className="ssl-result-validated-card-illustration">
            <img src={caIcon} loading="lazy" alt="" />
          </div>
          <div className="ssl-result-validated-card-content gap-xl">
            <div className="ssl-result-card-header">
              <h3>{m.initial_setup_auto_adoption_external_url_ssl_ca_title()}</h3>
              <p>{m.initial_setup_auto_adoption_external_url_ssl_ca_description()}</p>
            </div>
            <div>
              <Button
                text={m.initial_setup_auto_adoption_external_url_ssl_ca_download()}
                variant="outlined"
                iconLeft="download"
                onClick={handleDownloadCaCert}
                disabled={!sslInfoData?.ca_cert_pem}
              />
            </div>
          </div>
        </div>
      );
    }

    if (sslType === 'own_cert' && certInfo) {
      return (
        <div className="ssl-result-validated-card">
          <div className="ssl-result-validated-card-illustration">
            <img src={caIcon} loading="lazy" alt="" />
          </div>
          <div className="ssl-result-validated-card-content">
            <div className="ssl-result-card-header">
              <h3>{m.initial_setup_auto_adoption_external_url_ssl_own_title()}</h3>
              <p>{m.initial_setup_auto_adoption_external_url_ssl_own_description()}</p>
            </div>
            <div className="ssl-result-validated-card-info">
              <p className="ssl-result-card-info-title">
                {m.initial_setup_auto_adoption_external_url_ssl_own_info_title()}
              </p>
              <Divider />
              <div className="ssl-result-card-table">
                <div className="ssl-result-card-table-row">
                  <span className="label">
                    {m.initial_setup_auto_adoption_external_url_ssl_own_common_name()}
                  </span>
                  <span className="value">{certInfo.common_name}</span>
                </div>
                <div className="ssl-result-card-table-row">
                  <span className="label">
                    {m.initial_setup_auto_adoption_external_url_ssl_own_validity()}
                  </span>
                  <span className="value">
                    {m.initial_setup_auto_adoption_external_url_ssl_own_validity_days({
                      days: certInfo.valid_for_days,
                    })}
                  </span>
                </div>
              </div>
            </div>
          </div>
        </div>
      );
    }

    return null;
  };

  const isLetsEncryptProcessing = sslType === 'lets_encrypt' && acmeState.isProcessing;
  const isLetsEncryptIncomplete =
    sslType === 'lets_encrypt' && !acmeState.isComplete && !acmeState.errorMessage;
  const isLetsEncryptFailed = acmeState.isFailed;

  return (
    <WizardCard>
      {renderContent()}
      <SizedBox height={ThemeSpacing.Xl3} />
      <Controls>
        <div className="right">
          <Button
            text={m.controls_continue()}
            onClick={() => useSettingsEdgeCertificateWizardStore.getState().next()}
            disabled={
              isLetsEncryptProcessing || isLetsEncryptIncomplete || isLetsEncryptFailed
            }
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
