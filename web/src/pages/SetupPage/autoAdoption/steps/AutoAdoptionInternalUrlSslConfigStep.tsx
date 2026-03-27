import { useQuery } from '@tanstack/react-query';
import { useEffect } from 'react';
import { m } from '../../../../paraglide/messages';
import api from '../../../../shared/api/api';
import { Controls } from '../../../../shared/components/Controls/Controls';
import { WizardCard } from '../../../../shared/components/wizard/WizardCard/WizardCard';
import { Button } from '../../../../shared/defguard-ui/components/Button/Button';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { SizedBox } from '../../../../shared/defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';
import { downloadFile } from '../../../../shared/utils/download';
import caIcon from '../../assets/ca.png';
import { AutoAdoptionSetupStep } from '../types';
import { useAutoAdoptionSetupWizardStore } from '../useAutoAdoptionSetupWizardStore';
import './style.scss';

export const AutoAdoptionInternalUrlSslConfigStep = () => {
  const setActiveStep = useAutoAdoptionSetupWizardStore((s) => s.setActiveStep);
  const sslType = useAutoAdoptionSetupWizardStore((s) => s.internal_ssl_type);
  const certInfo = useAutoAdoptionSetupWizardStore((s) => s.internal_ssl_cert_info);
  // If ssl_type is not set (e.g. fresh browser session), redirect back so the
  // user can re-submit the settings step and repopulate the store.
  // biome-ignore lint/correctness/useExhaustiveDependencies: only run on mount
  useEffect(() => {
    if (sslType === null) {
      setActiveStep(AutoAdoptionSetupStep.InternalUrlSettings);
    }
  }, []);

  const { data: sslInfoData } = useQuery({
    queryKey: ['internal_ssl_info'],
    queryFn: () => api.initial_setup.getInternalSslInfo(),
    enabled: sslType === 'defguard_ca',
    select: (response) => response.data,
  });

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
              {m.initial_setup_auto_adoption_internal_url_ssl_no_ssl_title()}
            </h3>
            <p>{m.initial_setup_auto_adoption_internal_url_ssl_no_ssl_description()}</p>
          </div>
          <Divider />
          <ul className="ssl-port-list">
            <li>{m.initial_setup_auto_adoption_internal_url_ssl_no_ssl_port()}</li>
          </ul>
        </div>
      );
    }

    if (sslType === 'defguard_ca') {
      return (
        <div className="ssl-result-validated-card">
          <div className="ssl-result-validated-card-illustration">
            <img src={caIcon} loading="lazy" alt="" />
          </div>
          <div className="ssl-result-validated-card-content">
            <div className="ssl-result-card-header">
              <h3>{m.initial_setup_auto_adoption_internal_url_ssl_ca_title()}</h3>
              <p>{m.initial_setup_auto_adoption_internal_url_ssl_ca_description()}</p>
            </div>
            <div>
              <Button
                text={m.initial_setup_auto_adoption_internal_url_ssl_ca_download()}
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
              <h3>{m.initial_setup_auto_adoption_internal_url_ssl_own_title()}</h3>
              <p>{m.initial_setup_auto_adoption_internal_url_ssl_own_description()}</p>
            </div>
            <div className="ssl-result-validated-card-info">
              <p className="ssl-result-card-info-title">
                {m.initial_setup_auto_adoption_internal_url_ssl_own_info_title()}
              </p>
              <Divider />
              <div className="ssl-result-card-table">
                <div className="ssl-result-card-table-row">
                  <span className="label">
                    {m.initial_setup_auto_adoption_internal_url_ssl_own_common_name()}
                  </span>
                  <span className="value">{certInfo.common_name}</span>
                </div>
                <div className="ssl-result-card-table-row">
                  <span className="label">
                    {m.initial_setup_auto_adoption_internal_url_ssl_own_validity()}
                  </span>
                  <span className="value">
                    {m.initial_setup_auto_adoption_internal_url_ssl_own_validity_days({
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
      <SizedBox height={ThemeSpacing.Xl3} />
      <Divider />
      <Controls>
        <Button
          text={m.initial_setup_controls_back()}
          variant="outlined"
          onClick={() => setActiveStep(AutoAdoptionSetupStep.InternalUrlSettings)}
        />
        <div className="right">
          <Button
            text={m.initial_setup_controls_continue()}
            onClick={() => setActiveStep(AutoAdoptionSetupStep.ExternalUrlSettings)}
          />
        </div>
      </Controls>
    </WizardCard>
  );
};
