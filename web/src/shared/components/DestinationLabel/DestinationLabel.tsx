import { isPresent } from '../../defguard-ui/utils/isPresent';
import './style.scss';
import { Icon } from '../../defguard-ui/components/Icon';
import { SizedBox } from '../../defguard-ui/components/SizedBox/SizedBox';
import { ThemeSpacing } from '../../defguard-ui/types';
import type { DestinationLabelProps } from './types';
import { m } from '../../../paraglide/messages';

export const DestinationLabel = ({
  name,
  addresses,
  ports,
  protocols,
  anyAddress,
  anyPort,
  anyProtocol,
}: DestinationLabelProps) => {
  return (
    <div className="destination-label">
      <p className="name">{name}</p>
      <SizedBox height={1} width={ThemeSpacing.Sm} />
      <span className="separator">{`•`}</span>
      <SizedBox height={1} width={ThemeSpacing.Sm} />
      {(isPresent(ports) || anyPort) && (
        <>
          <Icon icon="globe" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info">{anyPort ? m.acl_destination_any_port() : ports}</span>
          <SizedBox height={1} width={ThemeSpacing.Md} />
        </>
      )}
      {(isPresent(protocols) || anyProtocol) && (
        <>
          <Icon icon="activity-notes" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info">
            {anyProtocol ? m.acl_destination_any_protocol() : protocols}
          </span>
          <SizedBox height={1} width={ThemeSpacing.Md} />
        </>
      )}
      {(isPresent(addresses) || anyAddress) && (
        <>
          <Icon icon="ip-suggest" />
          <SizedBox height={1} width={ThemeSpacing.Xs} />
          <span className="info wrap">{anyAddress ? m.acl_destination_any_address() : addresses}</span>
        </>
      )}
    </div>
  );
};
