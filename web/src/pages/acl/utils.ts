import { SelectOption } from '../../shared/defguard-ui/components/Layout/Select/types';
import { AclRuleInfo, Network } from '../../shared/types';
import { ListCellTag } from './AclIndexPage/components/shared/types';
import { AclAliasStatus, AclProtocol, AclStatus, NetworkAccessType } from './types';

// used by acl rules index page, bcs we don't show Applied in UI but instead enabled / disabled when state is "applied"
export const aclRuleToStatusInt = (rule: AclRuleInfo): number => {
  const status = rule.state;
  if (status === AclStatus.APPLIED) {
    if (rule.enabled) {
      return 1000;
    } else {
      return 999;
    }
  }
  return aclStatusToInt(rule.state);
};

export const aclStatusToInt = (status: AclStatus): number => {
  switch (status) {
    case AclStatus.NEW:
      return 0;
    case AclStatus.MODIFIED:
      return 1;
    case AclStatus.APPLIED:
      return 2;
    case AclStatus.DELETED:
      return 3;
    case AclStatus.EXPIRED:
      return 4;
  }
};

export const aclAliasStatusToInt = (status: AclAliasStatus): number => {
  switch (status) {
    case AclAliasStatus.APPLIED:
      return 2;
    case AclAliasStatus.MODIFIED:
      return 1;
  }
};

export const aclStatusFromInt = (statusInt: number): AclStatus => {
  switch (statusInt) {
    case 0:
      return AclStatus.NEW;
    case 1:
      return AclStatus.MODIFIED;
    case 2:
      return AclStatus.APPLIED;
    case 3:
      return AclStatus.DELETED;
    default:
      throw Error(`Mapping ACL Rule from int failed ! Unrecognized int of ${statusInt}`);
  }
};

export const aclAliasStatusFromInt = (statusInt: number): AclAliasStatus => {
  switch (statusInt) {
    case 1:
      return AclAliasStatus.APPLIED;
    case 2:
      return AclAliasStatus.MODIFIED;
    default:
      throw Error(`Unexpected alias status code of ${statusInt}`);
  }
};

export const protocolToString = (value: AclProtocol): string => {
  switch (value) {
    case AclProtocol.TCP:
      return 'TCP';
    case AclProtocol.UDP:
      return 'UDP';
    case AclProtocol.ICMP:
      return 'ICMP';
  }
};

export const protocolOptions: SelectOption<number>[] = [
  {
    key: AclProtocol.TCP,
    label: 'TCP',
    value: AclProtocol.TCP,
  },
  {
    key: AclProtocol.UDP,
    label: 'UDP',
    value: AclProtocol.UDP,
  },
  {
    key: AclProtocol.ICMP,
    label: 'ICMP',
    value: AclProtocol.ICMP,
  },
];

export const aclDestinationToListTagDisplay = (destination: string): ListCellTag[] =>
  destination
    .split(',')
    .filter((s) => s !== '')
    .map((dest, index) => ({
      key: `destination-${index}`,
      label: dest,
      displayAsTag: false,
    }));

export const aclPortsToListTagDisplay = (ports: string): ListCellTag[] =>
  ports
    .split(',')
    .filter((s) => s !== '')
    .map((port, index) => ({
      key: `port-${index}`,
      label: port,
      displayAsTag: false,
    }));

export const aclProtocolsToListTagDisplay = (protocols: AclProtocol[]): ListCellTag[] =>
  protocols.map((protocol) => ({
    key: protocol.toString(),
    label: protocolToString(protocol),
    displayAsTag: false,
  }));

export const aclRuleToListTagDisplay = (rules: AclRuleInfo[]): ListCellTag[] =>
  rules.map((rule) => ({
    key: rule.id,
    label: rule.name,
    displayAsTag: true,
  }));

export const networkToNetworkAccessType = (network: Network): NetworkAccessType => {
  if (!network.acl_enabled) {
    return NetworkAccessType.UNMANAGED;
  }
  if (network.acl_default_allow) {
    return NetworkAccessType.ALLOWED;
  } else {
    return NetworkAccessType.DENIED;
  }
};
