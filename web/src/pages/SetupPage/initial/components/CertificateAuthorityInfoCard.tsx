import { ActionCard } from '../../../../shared/components/ActionCard/ActionCard';
import { Divider } from '../../../../shared/defguard-ui/components/Divider/Divider';
import { ThemeSpacing } from '../../../../shared/defguard-ui/types';

type Props = {
  title: string;
  subtitle: string;
  infoTitle: string;
  commonNameLabel: string;
  validityLabel: string;
  commonName: string;
  validity: string;
  imageSrc: string;
};

export const CertificateAuthorityInfoCard = ({
  title,
  subtitle,
  infoTitle,
  commonNameLabel,
  validityLabel,
  commonName,
  validity,
  imageSrc,
}: Props) => {
  return (
    <ActionCard title={title} subtitle={subtitle} imageSrc={imageSrc}>
      <div className="ca-info">
        <p className="ca-info-title">{infoTitle}</p>
        <Divider spacing={ThemeSpacing.Md} />
        <div className="ca-info-grid">
          <div className="ca-info-label">{commonNameLabel}</div>
          <div className="ca-info-value">{commonName}</div>
          <div className="ca-info-label">{validityLabel}</div>
          <div className="ca-info-value">{validity}</div>
        </div>
      </div>
    </ActionCard>
  );
};
