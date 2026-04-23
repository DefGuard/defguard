import { m } from '../../../../paraglide/messages';
import type { CertInfo, InternalSslType } from '../../../api/types';
import { Button } from '../../../defguard-ui/components/Button/Button';
import { Divider } from '../../../defguard-ui/components/Divider/Divider';
import { isPresent } from '../../../defguard-ui/utils/isPresent';

type Props = {
  sslType: InternalSslType | null;
  certInfo: CertInfo | null;
  caCertPem?: string | null;
  onDownloadCaCert: () => void;
  imageSrc?: string;
};

export const InternalSslResult = ({
  sslType,
  certInfo,
  caCertPem,
  onDownloadCaCert,
  imageSrc,
}: Props) => {
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
          {isPresent(imageSrc) && <img src={imageSrc} loading="lazy" alt="" />}
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
              onClick={onDownloadCaCert}
              disabled={!caCertPem}
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
          {isPresent(imageSrc) && <img src={imageSrc} loading="lazy" alt="" />}
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
