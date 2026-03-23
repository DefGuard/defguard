import { useQuery } from '@tanstack/react-query';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { LoadingStep } from '../../../../shared/components/LoadingStep/LoadingStep';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import caIcon from '../../assets/ca.png';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

export const AutoAdoptionExternalUrlSslConfigStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const sslType = useAutoAdoptionSetupWizardStore((s) => s.external_ssl_type);
  const certInfo = useAutoAdoptionSetupWizardStore((s) => s.external_ssl_cert_info);

  const { data: sslInfoData } = useQuery({
    queryKey: ['external_ssl_info'],
    queryFn: () => api.initial_setup.getExternalSslInfo(),
    enabled: sslType === 'defguard_ca',
    select: (response) => response.data,
  });

  const handleDownloadCaCert = () => {
    if (!sslInfoData?.ca_cert_pem) return;
    const blob = new Blob([sslInfoData.ca_cert_pem], {
      type: 'application/x-pem-file',
    });
    const url = URL.createObjectURL(blob);
    const a = document.createElement('a');
    a.href = url;
    a.download = 'defguard-ca.pem';
    a.click();
    URL.revokeObjectURL(url);
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
            <li>
              {`Defguard Core service via `}
              <strong>TCP port 8000</strong>
            </li>
          </ul>
        </div>
      );
    }

    if (sslType === 'lets_encrypt') {
      const steps = [
        m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_connecting(),
        m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_validating(),
        m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_obtaining(),
        m.initial_setup_auto_adoption_external_url_ssl_lets_encrypt_installing(),
      ];
      return (
        <div className="ssl-result-card">
          <div>
            {steps.map((step) => (
              <LoadingStep key={step} title={step} success={true} />
            ))}
          </div>
          <Divider />
        </div>
      );
    }

    if (sslType === 'defguard_ca') {
      return (
        <div className="ssl-result-validated-card">
          <div className="ssl-result-validated-card-illustration">
            <img src={caIcon} loading="lazy" />
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
            <img src={caIcon} loading="lazy" />
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
                    {m.initial_setup_auto_adoption_external_url_ssl_own_name()}
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

  return (
    <WizardCard>
      {renderContent()}
      <Controls>
        <Button
          text={m.initial_setup_controls_back()}
          variant="outlined"
          onClick={() => setActiveStep(AutoAdoptionSetupStep.ExternalUrlSettings)}
        />
        <div className="right">
          <Button
            text={m.initial_setup_controls_continue()}
            onClick={() => setActiveStep(AutoAdoptionSetupStep.VpnSettings)}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
