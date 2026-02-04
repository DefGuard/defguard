import { isPresent } from '../../defguard-ui/utils/isPresent';
import './style.scss';
import { Icon } from '../../defguard-ui/components/Icon';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import type { DestinationLabelProps } from './types';

export const DestinationLabel = ({
  name,
  ips,
  ports,
  protocols,
}: DestinationLabelProps) => {
  return (
    <div className="destination-label">
      <p className="name">{name}</p>
      <SizedBox height={1} width={ThemeSpacing.Sm} />
      <span className="separator">{`•`}</span>
      <SizedBox height={1} width={ThemeSpacing.Sm} />
      {isPresent(ports) && (
        <>
          <Icon icon="globe" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info">{ports.length > 0 ? ports : `All ports`}</span>
          <SizedBox height={1} width={ThemeSpacing.Md} />
        </>
      )}
      {isPresent(protocols) && (
        <>
          <Icon icon="activity-notes" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info">
            {protocols.length > 0 ? protocols : `All protocols`}
          </span>
          <SizedBox height={1} width={ThemeSpacing.Md} />
        </>
      )}
      {isPresent(ips) && (
        <>
          <Icon icon="ip-suggest" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info wrap">{ips.length > 0 ? ips : `Any IP address`}</span>
        </>
      )}
    </div>
  );
};
